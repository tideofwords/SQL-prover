use clap::Parser;
use halo2_base::gates::{GateChip, GateInstructions};
use halo2_base::utils::ScalarField;
use halo2_base::AssignedValue;
#[allow(unused_imports)]
use halo2_base::{
    Context,
    QuantumCell::{Constant, Existing, Witness},
};
use halo2_scaffold::scaffold::cmd::Cli;
use halo2_scaffold::scaffold::run;
use poseidon::PoseidonChip;
use serde::{Deserialize, Serialize};

// standard parameters for Poseidon
const T: usize = 3;
const RATE: usize = 2;
const R_F: usize = 8;
const R_P: usize = 56;


pub trait Hashable<F: ScalarField> {
    fn fields(&self) -> &Vec<AssignedValue<F>>;
}

#[derive(Clone, Debug)]
pub struct TableRow<F: ScalarField> {
    fields: Vec<AssignedValue<F>>,
    max_fields: usize,
}

impl<F: ScalarField> TableRow<F> {
    fn zero(ctx: &mut Context<F>, max_fields: usize) -> TableRow<F> {
        let mut fields: Vec<AssignedValue<F>> = Vec::new();
        for i in 0..max_fields {
            fields.push(ctx.load_zero());
        }
        TableRow{fields, max_fields}
    }
}

impl<F: ScalarField> Hashable<F> for TableRow<F> {
    fn fields(&self) -> &Vec<AssignedValue<F>> {
        &self.fields
    }
}

#[derive(Clone, Debug)]
pub struct TableHead<F: ScalarField> {
    names: Vec<AssignedValue<F>>,
    max_fields: usize,
}

impl<F: ScalarField> Hashable<F> for TableHead<F> {
    fn fields(&self) -> &Vec<AssignedValue<F>> {
        &self.names
    }
}


#[derive(Clone, Debug)]
pub struct Table<F: ScalarField> {
    head: TableHead<F>,
    rows: Vec<TableRow<F>>,
    max_rows_bits: usize,
    max_fields: usize,
}

#[derive(Clone, Debug)]
pub struct TableWithHashes<F: ScalarField> {
    head: TableHead<F>,
    rows: Vec<TableRow<F>>,
    hashes: Vec<AssignedValue<F>>,
    max_rows_bits: usize,
    max_fields: usize,
}

impl<F: ScalarField> Table<F> {
    fn hash(&self, ctx: &mut Context<F>,) -> TableWithHashes<F> {
        let mut hashes: Vec<AssignedValue<F>> = Vec::new();
        hashes.push(hash_row(ctx, &self.head));
        for row in &self.rows {
            hashes.push(hash_row(ctx, row));
        }
        TableWithHashes {
            head: self.head.clone(),
            rows: self.rows.clone(),
            hashes,
            max_rows_bits: self.max_rows_bits,
            max_fields: self.max_fields,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableInput {
    pub head: [String; 3],
    pub rows: [[String; 3]; 6],
    pub max_fields: usize,
    pub max_rows_bits: usize,
}

#[derive(Clone, Debug)]
pub struct MerkleTree<F: ScalarField> {
    depth: usize, // num of rows will be 1 << depth
    head: TableHead<F>,
    rows: Vec<TableRow<F>>, // 0th is the head, the rest are rows, len is 2**depth
    hashes: Vec<AssignedValue<F>>, // len is 2**(depth+1)
    // do I also want to keep the tree of Merkle hashes?
}

#[derive(Clone, Debug)]
pub struct MerkleTreeWitness<F: ScalarField> {
    depth: usize,
    hashes: Vec<AssignedValue<F>>,
}

#[derive(Clone, Debug)]
pub struct MerkleRoot<F: ScalarField> {
    hash: AssignedValue<F>,
}

#[derive(Clone, Debug)]
pub struct Column<F: ScalarField> {
    values: Vec<AssignedValue<F>>,
}

fn hash_row<F: ScalarField>(
    ctx: &mut Context<F>,
    row: &impl Hashable<F>,
) -> AssignedValue<F> {
    let gate = GateChip::<F>::default();
    let mut poseidon = PoseidonChip::<F, T, RATE>::new(ctx, R_F, R_P).unwrap();
    poseidon.update(row.fields());
    poseidon.squeeze(ctx, &gate).unwrap()
}

fn constrain_permutation<F: ScalarField>(
    ctx: &mut Context<F>,
    arr1: &Vec<AssignedValue<F>>,
    arr2: &Vec<AssignedValue<F>>,
) {
    assert!(arr1.len() == arr2.len(), "Attempted permutation check on vectors of different lengths!");
    let gate = GateChip::<F>::default();

    // create challenge from Poseidon
    let mut poseidon = PoseidonChip::<F, T, RATE>::new(ctx, R_F, R_P).unwrap();
    poseidon.update(&arr1);
    poseidon.update(&arr2);
    let challenge = poseidon.squeeze(ctx, &gate).unwrap();

    // create grand products
    let mut grand_prod_1: Vec<AssignedValue<F>> = Vec::new();
    let mut grand_prod_2: Vec<AssignedValue<F>> = Vec::new();
    let mut cml_prod_1 = ctx.load_constant(F::one());
    let mut cml_prod_2 = ctx.load_constant(F::one());
    grand_prod_1.push(cml_prod_1);
    grand_prod_2.push(cml_prod_2);

    for idx in 0..arr1.len() {
        let temp_sum_1 = gate.add(ctx, challenge, arr1[idx]);
        cml_prod_1 = gate.mul(ctx, cml_prod_1, temp_sum_1);
        grand_prod_1.push(cml_prod_1);
        let temp_sum_2 = gate.add(ctx, challenge, arr2[idx]);
        cml_prod_2 = gate.mul(ctx, cml_prod_2, temp_sum_2);
        grand_prod_2.push(cml_prod_2);
    }

    // constrain equal
    ctx.constrain_equal(&grand_prod_1[arr1.len()], &grand_prod_2[arr1.len()]);
}

fn select<F: ScalarField>(
    ctx: &mut Context<F>,
    table: TableWithHashes<F>,
    sel_col: Column<F>,
) -> TableWithHashes<F> {
    let gate = GateChip::<F>::default();
    let sel = sel_col.values;
    // Compute selected inputs
    let mut out: Vec<TableRow<F>> = Vec::new();
    let mut out_hash: Vec<AssignedValue<F>> = Vec::new();
    let mut sel_out: Vec<AssignedValue<F>> = vec![table.hashes[0]]; // hash of TableHead stays the same
    let zero_row = TableRow::zero(ctx, table.max_fields);
    let hash_zero = hash_row(ctx, &zero_row);
    for i in 0..(table.rows.len()) {
        assert!(*sel[i].value() == F::one() || *sel[i].value() == F::zero());
        if *sel[i].value() == F::one() {
            out.push(table.rows[i].clone());
            sel_out.push(ctx.load_constant(F::from(1)));
            out_hash.push(hash_row(ctx, &table.rows[i]));
        }
    }

    for i in (out.len())..(table.rows.len()) {
        out.push(TableRow::zero(ctx, table.max_fields));
        out_hash.push(hash_row(ctx, &out[i-1]));
        sel_out.push(ctx.load_zero());
    }

    // Check that the rows are a permutation

    // hash_prod will include all the hashes to be selected,
    // and hash_zeros's as placeholders in the other positions
    let mut hash_prod: Vec<AssignedValue<F>> = Vec::new();
    for i in 0..(table.rows.len()) {
        hash_prod.push(gate.select(ctx, table.hashes[i], hash_zero, sel[i]));
    }

    constrain_permutation(ctx, &hash_prod, &out_hash);

    // Now out_hash is a permutation of hash_prod.
    // Need to constrain:
    // sel_out and sel are all 1's and 0's
    for i in 0..table.rows.len() {
        gate.assert_bit(ctx, sel[i]);
        gate.assert_bit(ctx, sel_out[i]);
    }
    // sel_out and sel have same num of 1's
    let mut cml_sum = ctx.load_zero();
    for i in 0..table.rows.len() {
        cml_sum = gate.add(ctx, cml_sum, sel[i]);
        cml_sum = gate.sub(ctx, cml_sum, sel_out[i]);
    }
    gate.assert_is_const(ctx, &cml_sum, &F::zero());
    // sel_out is 1's then 0's
    for i in 0..(table.rows.len() - 1) {
        let temp_prod = gate.mul(ctx, sel_out[i], sel_out[i+1]);
        ctx.constrain_equal(&sel_out[i+1], &temp_prod);
    }
    // out_hash[i] == out_hash[i] * sel_out[i]
    for i in 0..table.rows.len() {
        let temp_prod = gate.mul(ctx, out_hash[i], sel_out[i]);
        ctx.constrain_equal(&temp_prod, &out_hash[i]);
    }

    TableWithHashes {
        head: table.head,
        rows: out, 
        hashes: out_hash, 
        max_rows_bits: table.max_rows_bits, 
        max_fields: table.max_fields,
    }
}

fn strings_to_table<F: ScalarField>(
    ctx: &mut Context<F>,
    input: TableInput,
) -> Table<F> {
    let max_fields = input.max_fields;
    let max_rows_bits = input.max_rows_bits;
    let head = input.head.map(|s: String| ctx.load_witness(F::from_str_vartime(&s).unwrap()));
    let head = TableHead {
        names: head.to_vec(),
        max_fields,
    };
    let mut rows: Vec<TableRow<F>> = Vec::new();
    for input_row in input.rows {
        let row = input_row.map(|s: String| ctx.load_witness(F::from_str_vartime(&s).unwrap()));
        rows.push(TableRow {
            fields: row.to_vec(),
            max_fields,
        });
    }
    Table {
        head,
        rows,
        max_rows_bits,
        max_fields,
    }
}

fn strings_to_table_wrapper<F: ScalarField>(
    ctx: &mut Context<F>,
    input: TableInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    _ = strings_to_table(ctx, input);
}

fn main() {
    env_logger::init();

    let args = Cli::parse();
    run(strings_to_table_wrapper, args);
}



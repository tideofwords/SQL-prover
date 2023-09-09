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


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub x: [String; 10], // field element, but easier to deserialize as a string
    pub sel: [String; 10],
}

pub trait Hashable {
    type F: ScalarField;

    fn fields(&self) -> Vec<AssignedValue<Self::F>>;
}


#[derive(Clone, Debug)]
pub struct TableRow<F: ScalarField> {
    fields: Vec<AssignedValue<F>>,
    max_fields_bits: usize,
}

/* 
impl Hashable for TableRow<F: ScalarField> {
    type F = F;
    fn fields(&self) -> Vec<AssignedValue<F>> {
        self.fields
    }
}

pub trait Test {
    fn print(&self) -> ();
}

impl Test for TableRow<F: ScalarField> {
    fn print(&self) -> () {
        println!("testy testy");
    }
}
*/


#[derive(Clone, Debug)]
pub struct TableHead<F: ScalarField> {
    names: Vec<AssignedValue<F>>,
    max_fields_bits: usize,
}

/* 

impl Hashable<F> for TableHead<F: ScalarField> {
    fn fields(&self) -> Vec<AssignedValue<F>> {
        self.names
    }
}
*/


#[derive(Clone, Debug)]
pub struct Table<F: ScalarField> {
    head: TableHead<F>,
    rows: Vec<TableRow<F>>,
    max_rows_bits: usize,
}

#[derive(Clone, Debug)]
pub struct MerkleTree<F: ScalarField> {
    nodes: Vec<AssignedValue<F>>,
    // nodes[i] has children nodes[2*i + 1] and nodes[2*i + 2]
    depth: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleTestInput {
    pub head: [String; 3],
    pub rows: [[String; 3]; 6],
    pub max_fields_bits: usize,
    pub max_rows_bits: usize,
}

fn hash_row<F: ScalarField>(
    ctx: &mut Context<F>,
    row: TableRow<F>,
) -> AssignedValue<F> {
    let gate = GateChip::<F>::default();
    let mut poseidon = PoseidonChip::<F, T, RATE>::new(ctx, R_F, R_P).unwrap();
    poseidon.update(&row.fields);
    poseidon.squeeze(ctx, &gate).unwrap()
}


fn hash_head<F: ScalarField>(
    ctx: &mut Context<F>,
    head: TableHead<F>,
) -> AssignedValue<F> {
    let gate = GateChip::<F>::default();
    let mut poseidon = PoseidonChip::<F, T, RATE>::new(ctx, R_F, R_P).unwrap();
    poseidon.update(&head.names);
    poseidon.squeeze(ctx, &gate).unwrap()
}


fn hash_table<F: ScalarField>(
    ctx: &mut Context<F>,
    table: Table<F>,
) -> AssignedValue<F> {
    let gate = GateChip::<F>::default();
    let mut poseidon = PoseidonChip::<F, T, RATE>::new(ctx, R_F, R_P).unwrap();
    poseidon.update(&table.head.names);
    for row in table.rows {
        poseidon.update(&row.fields);
    }
    poseidon.squeeze(ctx, &gate).unwrap()
}

fn hash_two<F: ScalarField>(
    ctx: &mut Context<F>,
    a: &AssignedValue<F>,
    b: &AssignedValue<F>,
) -> AssignedValue<F> {
    let gate = GateChip::<F>::default();
    let mut poseidon = PoseidonChip::<F, T, RATE>::new(ctx, R_F, R_P).unwrap();
    poseidon.update(&[*a, *b]);
    poseidon.squeeze(ctx, &gate).unwrap()
}

fn merkelize<F: ScalarField>(
    ctx: &mut Context<F>,
    table: Table<F>,
) -> MerkleTree<F> {
    // table is required to have (2^k - 1) rows,
    // some of which can be 0 (i.e. null)

    // the output Merkle tree has depth k+1

    let mut tree_layers: Vec<Vec<AssignedValue<F>>> = Vec::new();
    let depth = table.max_rows_bits + 1;
    for i in 0..depth {
        tree_layers.push(Vec::new());
    }

    assert!(table.rows.len() <= (1 << table.max_rows_bits) - 1);

    tree_layers[0].push(hash_head(ctx, table.head));
    for row in table.rows {
        tree_layers[0].push(hash_row(ctx, row));
    }

    for _ in tree_layers[0].len()..(1 << table.max_rows_bits) {
        tree_layers[0].push(ctx.load_zero());
    }

    for i in 1..depth {
        let layer_size = tree_layers[i-1].len();
        for j in 0..(layer_size / 2) {
            let next_item = hash_two(ctx, &tree_layers[i-1][2*j], &tree_layers[i-1][2*j + 1]);
            tree_layers[i].push(next_item);
        }
    }

    let mut nodes: Vec<AssignedValue<F>> = Vec::new();
    // rearrange tree_layers into Merkle tree -- or maybe just keep this order!?
    for i in (0..depth).rev() {
        nodes.extend(&tree_layers[i]);
    }

    MerkleTree {nodes, depth}
}

fn select<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    let x = input.x.map(|s: String| ctx.load_witness(F::from_str_vartime(&s).unwrap()));
    let sel = input.sel.map(|s: String| ctx.load_witness(F::from_str_vartime(&s).unwrap()));

    let gate = GateChip::<F>::default();


    // compute selected inputs
    let mut out: Vec<AssignedValue<F>> = Vec::new();
    let mut sel_out: Vec<AssignedValue<F>> = Vec::new();
    for i in 0..10 {
        if *sel[i].value() == F::one() {
            out.push(x[i]);
            sel_out.push(ctx.load_constant(F::from(1)));
        }
    }
    for i in out.len()..10 {
        out.push(ctx.load_zero());
        sel_out.push(ctx.load_zero());
    }

    // compute products
    let prods: Vec<AssignedValue<F>> = x.iter().zip(sel.iter()).map(
        |(x, y)| gate.mul(ctx, *x, *y)
    ).collect();

    // check out is a permutation of prods, and
    // check sel_out is a permutation of sel
    // using grand product argument, with 'challenge' as the challenge

    // first, get Poseidon hash
    let mut poseidon = PoseidonChip::<F, T, RATE>::new(ctx, R_F, R_P).unwrap();
    poseidon.update(&out);
    poseidon.update(&prods);
    poseidon.update(&sel);
    poseidon.update(&sel_out);
    let challenge = poseidon.squeeze(ctx, &gate).unwrap();

    // compute grand product

    let mut grand_prod_out: Vec<AssignedValue<F>> = Vec::new();
    let mut grand_prod_prods: Vec<AssignedValue<F>> = Vec::new();
    let mut grand_prod_sel: Vec<AssignedValue<F>> = Vec::new();
    let mut grand_prod_sel_out: Vec<AssignedValue<F>> = Vec::new();

    let mut cml_prod_out = ctx.load_constant(F::one());
    let mut cml_prod_prods = ctx.load_constant(F::one());
    let mut cml_prod_sel = ctx.load_constant(F::one());
    let mut cml_prod_sel_out = ctx.load_constant(F::one());

    grand_prod_out.push(cml_prod_out);
    grand_prod_prods.push(cml_prod_prods);
    grand_prod_sel.push(cml_prod_sel);
    grand_prod_sel_out.push(cml_prod_sel_out);

    for i in 0..out.len() {
        let temp_sum = gate.add(
            ctx,
            out[i],
            challenge,
        );
        cml_prod_out = gate.mul(
            ctx,
            cml_prod_out,
            temp_sum,
        );

        let temp_sum = gate.add(
            ctx,
            prods[i],
            challenge,
        );
        cml_prod_prods = gate.mul(
            ctx,
            cml_prod_prods,
            temp_sum,
        );

        let temp_sum = gate.add(
            ctx,
            sel[i],
            challenge,
        );
        cml_prod_sel = gate.mul(
            ctx,
            cml_prod_sel,
            temp_sum,
        );

        let temp_sum = gate.add(
            ctx,
            sel_out[i],
            challenge,
        );
        cml_prod_sel_out = gate.mul(
            ctx,
            cml_prod_sel_out,
            temp_sum,
        );
        grand_prod_out.push(cml_prod_out);
        grand_prod_prods.push(cml_prod_prods);
        grand_prod_sel.push(cml_prod_sel);
        grand_prod_sel_out.push(cml_prod_sel_out);
    }

    ctx.constrain_equal(&grand_prod_out[out.len()], &grand_prod_prods[out.len()]);
    ctx.constrain_equal(&grand_prod_sel[out.len()], &grand_prod_sel_out[out.len()]);
    
    // check sel_out is a bunch of ones followed by a bunch of zeros
    for i in 0..(out.len()-1) {
        let temp_prod = gate.mul(
            ctx,
            sel_out[i],
            sel_out[i+1],
        );
        ctx.constrain_equal(
            &sel_out[i+1],
            &temp_prod,
        );

    }

    // check out[i] * sel_out[i] == out[i]
    // and sel_out[i] is 0 or 1
    for i in 0..out.len() {
        let temp_prod = gate.mul(
            ctx,
            sel_out[i],
            sel_out[i],
        );
        ctx.constrain_equal(
            &sel_out[i],
            &temp_prod,
        );

        let temp_prod = gate.mul(
            ctx,
            out[i],
            sel_out[i],
        );
        ctx.constrain_equal(
            &out[i],
            &temp_prod,
        );
    }


}

fn strings_to_table<F: ScalarField>(
    ctx: &mut Context<F>,
    input: MerkleTestInput,
) -> Table<F> {
    let max_fields_bits = input.max_fields_bits;
    let max_rows_bits = input.max_rows_bits;
    let head = input.head.map(|s: String| ctx.load_witness(F::from_str_vartime(&s).unwrap()));
    let head = TableHead {
        names: head.to_vec(),
        max_fields_bits,
    };
    let mut rows: Vec<TableRow<F>> = Vec::new();
    for input_row in input.rows {
        let row = input_row.map(|s: String| ctx.load_witness(F::from_str_vartime(&s).unwrap()));
        rows.push(TableRow {
            fields: row.to_vec(),
            max_fields_bits,
        });
    }
    Table {
        head,
        rows,
        max_rows_bits,
    }
}

fn merkle_wrapper<F: ScalarField>(
    ctx: &mut Context<F>,
    input: MerkleTestInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // Convert MerkleTestInput into Table
    let table = strings_to_table(ctx, input);

    // Merkelize it
    let tree = merkelize(ctx, table);

}

fn main() {
    env_logger::init();

    let args = Cli::parse();
    run(merkle_wrapper, args);
}


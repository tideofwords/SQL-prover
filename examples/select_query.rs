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

const T: usize = 3;
const RATE: usize = 2;
const R_F: usize = 8;
const R_P: usize = 57;

// const DB_COLUMNS: usize = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub db: Vec<u64>,
    pub indices: Vec<u64>,
}

// select indices from a database given a vector of indices
fn select_query<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    let db = ctx.assign_witnesses(input.db.into_iter().map(F::from));
    let indices = ctx.assign_witnesses(input.indices.into_iter().map(F::from));
    let mut poseidon = PoseidonChip::<F, T, RATE>::new(ctx, R_F, R_P).unwrap();

    assert_eq!(db.len(), indices.len());
    let length = db.len();

    let gate = GateChip::<F>::default();
    for i in 0..length {
        gate.assert_bit(ctx, indices[i]);
    }

    let mut out = vec![];
    for i in 0..length {
        out.push(gate.mul(ctx, db[i], indices[i]));
    }

    poseidon.update(&db);
    let hash = poseidon.squeeze(ctx, &gate).unwrap();

    make_public.extend(&db);
    make_public.extend(&indices);
    make_public.extend(&out);
    make_public.push(hash);

    println!("database: {:?}", db.iter().map(|x| *x.value()).collect::<Vec<F>>());
    println!("indices: {:?}", indices.iter().map(|x| *x.value()).collect::<Vec<F>>());
    println!("out: {:?}", out.iter().map(|x| *x.value()).collect::<Vec<F>>());
    println!("hash: {:?}", hash.value());
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(select_query, args);
}

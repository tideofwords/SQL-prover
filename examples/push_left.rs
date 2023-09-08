use clap::Parser;
use halo2_base::gates::{GateChip, GateInstructions};
use halo2_base::utils::ScalarField;
use halo2_base::AssignedValue;
#[allow(unused_imports)]
use halo2_base::{
    Context,
    QuantumCell::{Constant, Existing, Witness},
};
use halo2_proofs::dev::metadata::Gate;
use halo2_scaffold::scaffold::cmd::Cli;
use halo2_scaffold::scaffold::run;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub arr: Vec<u64>,
    pub indices: Vec<u64>,
}

// pushes all nonzero elements to the left
fn push_left_one<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    let arr = ctx.assign_witnesses(input.arr.into_iter().map(F::from));
    let indices = ctx.assign_witnesses(input.indices.into_iter().map(F::from));

    assert_eq!(arr.len(), indices.len());
    let length = arr.len();

    make_public.extend(&arr);
    make_public.extend(&indices);
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(push_left_one, args);
}

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
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub arr: Vec<u64>,
    pub indices: Vec<u64>,
    pub gamma: u64,
}

fn some_algorithm_in_zk<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    let arr = ctx.assign_witnesses(input.arr.into_iter().map(F::from));
    let indices = ctx.assign_witnesses(input.indices.into_iter().map(F::from));
    let gamma = ctx.load_witness(F::from(input.gamma));

    assert_eq!(arr.len(), indices.len());
    let length = arr.len();

    make_public.extend(&arr);
    make_public.extend(&indices);
    make_public.push(gamma);

    let mut output = vec![];

    let gate = GateChip::default();
    let gamma_minus_one = gate.sub(ctx, gamma, Constant(F::one()));
    let mut rlc = ctx.load_constant(F::zero());

    for (a, i) in arr.into_iter().zip(indices) {
        if *i.value() == F::one() {
            output.push(a);
        }
        let intermediate = gate.mul_add(ctx, i, gamma_minus_one, Constant(F::one()));
        rlc = gate.mul_add(ctx, rlc, intermediate, a);
    }

    output.resize(length, ctx.load_witness(F::zero()));
    make_public.extend(&output);

    println!("rlc: {:?}", rlc.value());
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(some_algorithm_in_zk, args);
}

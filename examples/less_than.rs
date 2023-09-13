use axiom_eth;
use clap::Parser;
use halo2_base::gates::{GateChip, GateInstructions};
use halo2_base::safe_types::{RangeChip, RangeInstructions};
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
use std::env::var;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub arr: Vec<String>,
    pub val: String,
}

fn less_than<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    let lookup_bits =
        var("LOOKUP_BITS").unwrap_or_else(|_| panic!("LOOKUP_BITS not set")).parse().unwrap();
    let arr = ctx.assign_witnesses(input.arr.iter().map(|b| F::from_str_vartime(b).unwrap()));
    let val = ctx.load_witness(F::from_str_vartime(&input.val).unwrap());

    let range = RangeChip::default(lookup_bits);
    let out: Vec<AssignedValue<F>> =
        arr.iter().map(|&x| range.is_less_than(ctx, x, val, 10)).collect();

    make_public.extend(&arr);
    make_public.push(val);
    make_public.extend(&out);

    println!("out: {:?}", out);
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(less_than, args);
}

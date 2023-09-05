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

// these parameters are ad hoc and need to be checked
const T: usize = 3;
const RATE: usize = 2;
const R_F: usize = 8;
const R_P: usize = 57;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub x: [String; 10], // field element, but easier to deserialize as a string
    pub sel: [String; 10],
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

fn main() {
    env_logger::init();

    let args = Cli::parse();
    run(select, args);
}


use halo2_base::gates::{GateChip, GateInstructions};
use halo2_base::halo2_proofs::halo2curves::bn256::Fr;
use halo2_base::utils::ScalarField;
use halo2_base::AssignedValue;
use halo2_base::{
    Context,
    QuantumCell::{Constant, Existing, Witness},
};
use halo2_proofs::arithmetic::Field;
#[allow(unused_imports)]
use halo2_scaffold::scaffold::{mock, prove};
use rand::rngs::OsRng;

fn some_algorithm_in_zk<F: ScalarField>(
    ctx: &mut Context<F>,
    x: F,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // `Context` can roughly be thought of as a single-threaded execution trace of a program we want to ZK prove. We do some post-processing on `Context` to optimally divide the execution trace into multiple columns in a PLONKish arithmetization
    // More advanced usage with multi-threaded witness generation is possible, but we do not explain it here

    // first we load a number `x` into as system, as a "witness"
    let x = ctx.load_witness(x);
    // by default, all numbers in the system are private
    // we can make it public like so:
    make_public.push(x);

    // create a Gate chip that contains methods for basic arithmetic operations
    let gate = GateChip::<F>::default();

    // ===== way 1 =====
    // now we can perform arithmetic operations almost like a normal program using halo2-lib API functions
    // square x
    let x_sq = gate.mul(ctx, x, x);

    // x^2 + 72
    let c = F::from(72);
    // the implicit type of most variables is an "Existing" assigned value
    // a known constant is a separate type that we specify by `Constant(c)`:
    let out = gate.add(ctx, x_sq, Constant(c));
    // Halo2 does not distinguish between public inputs vs outputs because the verifier seems them all at the same time
    // However in traditional terms, `out` is our output number. It is currently still private.
    // Let's make it public:
    make_public.push(out);

    // ==== way 2 =======
    // here is a more optimal way to compute x^2 + 72 using the lower level `assign_region` API:
    let val = *x.value() * x.value() + c;
    let _val_assigned =
        ctx.assign_region_last([Constant(c), Existing(x), Existing(x), Witness(val)], [0]);
    // the `[0]` tells us to turn on a vertical `a + b * c = d` gate at row position 0.
    // this imposes the constraint c + x * x = val

    // ==== way 3 ======
    // this does the exact same thing as way 2, but with a pre-existing function
    let _val_assigned = gate.mul_add(ctx, x, x, Constant(c));
    println!("x: {:?}", x.value());
    println!("val_assigned: {:?}", _val_assigned.value());
    assert_eq!(*x.value() * x.value() + c, *_val_assigned.value());
}

fn main() {
    env_logger::init();

    // we create a random public input
    let x = Fr::random(OsRng);
    // run mock prover
    mock(some_algorithm_in_zk, x);

    // uncomment below to run actual prover:
    // prove(some_algorithm_in_zk, x, Fr::zero()); // the Fr::zero() is a dummy input to provide for the proving key generation
}
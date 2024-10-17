// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::ops::Neg;

use core::f64::consts::FRAC_1_SQRT_2;
use nalgebra;

use noisy_simulator::{operation, Instrument, NoisySimulator, Operation, StateVectorSimulator};
use num_bigint::BigUint;
use num_complex::Complex;
use qsc_hir::mut_visit;
use quantum_sparse_sim::QuantumSim;
use rand::RngCore;

use crate::val::{Qubit, Value};

/// The trait that must be implemented by a quantum backend, whose functions will be invoked when
/// quantum intrinsics are called.
pub trait Backend {
    type ResultType;

    fn ccx(&mut self, _ctl0: usize, _ctl1: usize, _q: usize) {
        unimplemented!("ccx gate");
    }
    fn cx(&mut self, _ctl: usize, _q: usize) {
        unimplemented!("cx gate");
    }
    fn cy(&mut self, _ctl: usize, _q: usize) {
        unimplemented!("cy gate");
    }
    fn cz(&mut self, _ctl: usize, _q: usize) {
        unimplemented!("cz gate");
    }
    fn h(&mut self, _q: usize) {
        unimplemented!("h gate");
    }
    fn m(&mut self, _q: usize) -> Self::ResultType {
        unimplemented!("m operation");
    }
    fn mresetz(&mut self, _q: usize) -> Self::ResultType {
        unimplemented!("mresetz operation");
    }
    fn reset(&mut self, _q: usize) {
        unimplemented!("reset gate");
    }
    fn rx(&mut self, _theta: f64, _q: usize) {
        unimplemented!("rx gate");
    }
    fn rxx(&mut self, _theta: f64, _q0: usize, _q1: usize) {
        unimplemented!("rxx gate");
    }
    fn ry(&mut self, _theta: f64, _q: usize) {
        unimplemented!("ry gate");
    }
    fn ryy(&mut self, _theta: f64, _q0: usize, _q1: usize) {
        unimplemented!("ryy gate");
    }
    fn rz(&mut self, _theta: f64, _q: usize) {
        unimplemented!("rz gate");
    }
    fn rzz(&mut self, _theta: f64, _q0: usize, _q1: usize) {
        unimplemented!("rzz gate");
    }
    fn sadj(&mut self, _q: usize) {
        unimplemented!("sadj gate");
    }
    fn s(&mut self, _q: usize) {
        unimplemented!("s gate");
    }
    fn swap(&mut self, _q0: usize, _q1: usize) {
        unimplemented!("swap gate");
    }
    fn tadj(&mut self, _q: usize) {
        unimplemented!("tadj gate");
    }
    fn t(&mut self, _q: usize) {
        unimplemented!("t gate");
    }
    fn x(&mut self, _q: usize) {
        unimplemented!("x gate");
    }
    fn y(&mut self, _q: usize) {
        unimplemented!("y gate");
    }
    fn z(&mut self, _q: usize) {
        unimplemented!("z gate");
    }
    fn qubit_allocate(&mut self) -> usize {
        unimplemented!("qubit_allocate operation");
    }
    fn qubit_release(&mut self, _q: usize) {
        unimplemented!("qubit_release operation");
    }
    fn qubit_swap_id(&mut self, _q0: usize, _q1: usize) {
        unimplemented!("qubit_swap_id operation");
    }
    fn capture_quantum_state(&mut self) -> (Vec<(BigUint, Complex<f64>)>, usize) {
        unimplemented!("capture_quantum_state operation");
    }
    fn qubit_is_zero(&mut self, _q: usize) -> bool {
        unimplemented!("qubit_is_zero operation");
    }
    fn custom_intrinsic(&mut self, _name: &str, _arg: Value) -> Option<Result<Value, String>> {
        None
    }
    fn set_seed(&mut self, _seed: Option<u64>) {}
}

/// Default backend used when targeting sparse simulation.
pub struct SparseSim {
    pub sim: QuantumSim,
}

impl Default for SparseSim {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseSim {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sim: QuantumSim::new(None),
        }
    }
}

impl Backend for SparseSim {
    type ResultType = bool;

    fn ccx(&mut self, ctl0: usize, ctl1: usize, q: usize) {
        self.sim.mcx(&[ctl0, ctl1], q);
    }

    fn cx(&mut self, ctl: usize, q: usize) {
        self.sim.mcx(&[ctl], q);
    }

    fn cy(&mut self, ctl: usize, q: usize) {
        self.sim.mcy(&[ctl], q);
    }

    fn cz(&mut self, ctl: usize, q: usize) {
        self.sim.mcz(&[ctl], q);
    }

    fn h(&mut self, q: usize) {
        self.sim.h(q);
    }

    fn m(&mut self, q: usize) -> Self::ResultType {
        self.sim.measure(q)
    }

    fn mresetz(&mut self, q: usize) -> Self::ResultType {
        let res = self.sim.measure(q);
        if res {
            self.sim.x(q);
        }
        res
    }

    fn reset(&mut self, q: usize) {
        self.mresetz(q);
    }

    fn rx(&mut self, theta: f64, q: usize) {
        self.sim.rx(theta, q);
    }

    fn rxx(&mut self, theta: f64, q0: usize, q1: usize) {
        self.h(q0);
        self.h(q1);
        self.rzz(theta, q0, q1);
        self.h(q1);
        self.h(q0);
    }

    fn ry(&mut self, theta: f64, q: usize) {
        self.sim.ry(theta, q);
    }

    fn ryy(&mut self, theta: f64, q0: usize, q1: usize) {
        self.h(q0);
        self.s(q0);
        self.h(q0);
        self.h(q1);
        self.s(q1);
        self.h(q1);
        self.rzz(theta, q0, q1);
        self.h(q1);
        self.sadj(q1);
        self.h(q1);
        self.h(q0);
        self.sadj(q0);
        self.h(q0);
    }

    fn rz(&mut self, theta: f64, q: usize) {
        self.sim.rz(theta, q);
    }

    fn rzz(&mut self, theta: f64, q0: usize, q1: usize) {
        self.cx(q1, q0);
        self.rz(theta, q0);
        self.cx(q1, q0);
    }

    fn sadj(&mut self, q: usize) {
        self.sim.sadj(q);
    }

    fn s(&mut self, q: usize) {
        self.sim.s(q);
    }

    fn swap(&mut self, q0: usize, q1: usize) {
        self.sim.swap_qubit_ids(q0, q1);
    }

    fn tadj(&mut self, q: usize) {
        self.sim.tadj(q);
    }

    fn t(&mut self, q: usize) {
        self.sim.t(q);
    }

    fn x(&mut self, q: usize) {
        self.sim.x(q);
    }

    fn y(&mut self, q: usize) {
        self.sim.y(q);
    }

    fn z(&mut self, q: usize) {
        self.sim.z(q);
    }

    fn qubit_allocate(&mut self) -> usize {
        self.sim.allocate()
    }

    fn qubit_release(&mut self, q: usize) {
        self.sim.release(q);
    }

    fn qubit_swap_id(&mut self, q0: usize, q1: usize) {
        self.sim.swap_qubit_ids(q0, q1);
    }

    fn capture_quantum_state(&mut self) -> (Vec<(BigUint, Complex<f64>)>, usize) {
        let (state, count) = self.sim.get_state();
        // Because the simulator returns the state indices with opposite endianness from the
        // expected one, we need to reverse the bit order of the indices.
        let mut new_state = state
            .into_iter()
            .map(|(idx, val)| {
                let mut new_idx = BigUint::default();
                for i in 0..(count as u64) {
                    if idx.bit((count as u64) - 1 - i) {
                        new_idx.set_bit(i, true);
                    }
                }
                (new_idx, val)
            })
            .collect::<Vec<_>>();
        new_state.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        (new_state, count)
    }

    fn qubit_is_zero(&mut self, q: usize) -> bool {
        self.sim.qubit_is_zero(q)
    }

    fn custom_intrinsic(&mut self, name: &str, arg: Value) -> Option<Result<Value, String>> {
        match name {
            "GlobalPhase" => {
                // Apply a global phase to the simulation by doing an Rz to a fresh qubit.
                // The controls list may be empty, in which case the phase is applied unconditionally.
                let [ctls_val, theta] = &*arg.unwrap_tuple() else {
                    panic!("tuple arity for GlobalPhase intrinsic should be 2");
                };
                let ctls = ctls_val
                    .clone()
                    .unwrap_array()
                    .iter()
                    .map(|q| q.clone().unwrap_qubit().0)
                    .collect::<Vec<_>>();
                let q = self.sim.allocate();
                // The new qubit is by-definition in the |0⟩ state, so by reversing the sign of the
                // angle we can apply the phase to the entire state without increasing its size in memory.
                self.sim
                    .mcrz(&ctls, -2.0 * theta.clone().unwrap_double(), q);
                self.sim.release(q);
                Some(Ok(Value::unit()))
            }
            "BeginEstimateCaching" => Some(Ok(Value::Bool(true))),
            "EndEstimateCaching"
            | "AccountForEstimatesInternal"
            | "BeginRepeatEstimatesInternal"
            | "EndRepeatEstimatesInternal" => Some(Ok(Value::unit())),
            _ => None,
        }
    }

    fn set_seed(&mut self, seed: Option<u64>) {
        match seed {
            Some(seed) => self.sim.set_rng_seed(seed),
            None => self.sim.set_rng_seed(rand::thread_rng().next_u64()),
        }
    }
}

pub struct QubitManager {
    // TODO: This implementation is just to test the noisy simulator approach.
    // If used, replace with the optimized implementation.
    pub user_id_to_sim_id: Vec<usize>,
    pub is_free: Vec<bool>,
    pub allocated_count: usize,
}

impl QubitManager {
    #[must_use]
    pub fn new(max_allowed_number_of_qubits: usize) -> Self {
        Self {
            user_id_to_sim_id: (0..max_allowed_number_of_qubits).collect(),
            is_free: vec![true; max_allowed_number_of_qubits],
            allocated_count: 0,
        }
    }
    pub fn allocate(&mut self) -> Option<usize> {
        for id in 0..self.is_free.len() {
            if self.is_free[id] {
                self.is_free[id] = false;
                self.allocated_count += 1;
                return Some(id);
            }
        }
        None
    }
    pub fn release(&mut self, id: usize) {
        assert!(!self.is_free[id]);
        self.is_free[id] = true;
        self.allocated_count -= 1;
    }
    pub fn swap(&mut self, id1: usize, id2: usize) {
        assert!(!self.is_free[id1]);
        assert!(!self.is_free[id2]);
        self.user_id_to_sim_id.swap(id1, id2);
    }
    #[must_use]
    pub fn map(&self, id: usize) -> usize {
        self.user_id_to_sim_id[id]
    }
    #[must_use]
    pub fn qubit_count(&self) -> usize {
        self.allocated_count
    }
}

pub struct StateVectorNoisySim {
    pub max_qubit_id: usize, // Max user qubit id seen so far.
    pub sim: StateVectorSimulator,
    pub qman: QubitManager,
    pub ccx_op: Operation,
    pub cx_op: Operation,
    pub cy_op: Operation,
    pub cz_op: Operation,
    pub h_op: Operation,
    pub z_inst: Instrument,
    pub sadj_op: Operation,
    pub s_op: Operation,
    pub swap_op: Operation,
    pub tadj_op: Operation,
    pub t_op: Operation,
    pub x_op: Operation,
    pub y_op: Operation,
    pub z_op: Operation,
    pub reset_inst: Instrument,
}

impl StateVectorNoisySim {
    #[must_use]
    pub fn new(number_of_qubits: usize) -> Self {
        // TODO: Implement proper matricies
        Self {
            max_qubit_id: 0,
            sim: StateVectorSimulator::new(number_of_qubits),
            qman: QubitManager::new(number_of_qubits),
            ccx_op: operation!(
                [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0;
                 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0;
                 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0;
                 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0;
                 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0;
                 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0;
                 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0;
                 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]
            )
            .unwrap(),
            cx_op: operation!(
                [1.0, 0.0, 0.0, 0.0;
                 0.0, 1.0, 0.0, 0.0;
                 0.0, 0.0, 0.0, 1.0;
                 0.0, 0.0, 1.0, 0.0])
            .unwrap(),
            cy_op: operation!(
                [1.0, 0.0, 0.0, 0.0;
                 0.0, 1.0, 0.0, 0.0;
                 0.0, 0.0, 0.0, Complex::i().neg();
                 0.0, 0.0, Complex::i(), 0.0])
            .unwrap(),
            cz_op: operation!(
                [1.0, 0.0, 0.0, 0.0;
                 0.0, 1.0, 0.0, 0.0;
                 0.0, 0.0, 1.0, 0.0;
                 0.0, 0.0, 0.0, -1.0])
            .unwrap(),
            h_op: operation!(
                [FRAC_1_SQRT_2,  FRAC_1_SQRT_2 ;
                 FRAC_1_SQRT_2, -FRAC_1_SQRT_2])
            .unwrap(),
            z_inst: Instrument::new(vec![
                operation!(
                    [1.0, 0.0;
                     0.0, 0.0])
                .unwrap(),
                operation!(
                    [0.0, 0.0;
                     0.0, 1.0])
                .unwrap(),
            ])
            .unwrap(),
            sadj_op: operation!(
                [1.0, 0.0;
                 0.0, Complex::i().neg()]
            )
            .unwrap(),
            s_op: operation!(
                [1.0, 0.0;
                 0.0, Complex::i()]
            )
            .unwrap(),
            swap_op: operation!(
                [1.0, 0.0, 0.0, 0.0;
                 0.0, 0.0, 1.0, 0.0;
                 0.0, 1.0, 0.0, 0.0;
                 0.0, 0.0, 0.0, 1.0])
            .unwrap(),
            tadj_op: operation!(
                [1.0, 0.0;
                 0.0, Complex::new(0.707_106_781_186_547_5, -0.707_106_781_186_547_5)]
            )
            .unwrap(),
            t_op: operation!(
                [1.0, 0.0;
                 0.0, Complex::new(0.707_106_781_186_547_5, 0.707_106_781_186_547_5)]
            )
            .unwrap(),
            x_op: operation!(
                [0.0, 1.0;
                 1.0, 0.0]
            )
            .unwrap(),
            y_op: operation!(
                [0.0, Complex::i().neg();
                 Complex::i(), 0.0]
            )
            .unwrap(),
            z_op: operation!(
                [1.0, 0.0;
                 0.0, -1.0]
            )
            .unwrap(),
            reset_inst: Instrument::new(vec![
                operation!(
                    [1.0, 0.0;
                     0.0, 0.0])
                .unwrap(),
                operation!(
                    [0.0, 1.0;
                     0.0, 0.0])
                .unwrap(),
            ])
            .unwrap(),
        }
    }
}

impl Backend for StateVectorNoisySim {
    type ResultType = bool;

    fn ccx(&mut self, ctl0: usize, ctl1: usize, q: usize) {
        self.sim
            .apply_operation(
                &self.ccx_op,
                &[self.qman.map(q), self.qman.map(ctl1), self.qman.map(ctl0)],
            )
            .expect("Noisy ccx failed.");
    }
    fn cx(&mut self, ctl: usize, q: usize) {
        self.sim
            .apply_operation(&self.cx_op, &[self.qman.map(q), self.qman.map(ctl)])
            .expect("Noisy cx failed.");
    }
    fn cy(&mut self, ctl: usize, q: usize) {
        self.sim
            .apply_operation(&self.cy_op, &[self.qman.map(q), self.qman.map(ctl)])
            .expect("Noisy cy failed.");
    }
    fn cz(&mut self, ctl: usize, q: usize) {
        self.sim
            .apply_operation(&self.cz_op, &[self.qman.map(q), self.qman.map(ctl)])
            .expect("Noisy cz failed.");
    }
    fn h(&mut self, q: usize) {
        self.sim
            .apply_operation(&self.h_op, &[self.qman.map(q)])
            .expect("Noisy h failed.");
    }
    fn m(&mut self, q: usize) -> Self::ResultType {
        let outcome = self
            .sim
            .sample_instrument(&self.z_inst, &[self.qman.map(q)])
            .expect("Noisy m failed");
        outcome != 0
    }
    fn mresetz(&mut self, q: usize) -> Self::ResultType {
        let result = self.m(q);
        if result {
            self.x(q);
        }
        result
    }
    fn reset(&mut self, q: usize) {
        self.sim
            .apply_instrument(&self.reset_inst, &[self.qman.map(q)])
            .expect("Noisy reset failed.");
    }
    fn rx(&mut self, theta: f64, q: usize) {
        let t = theta / 2.0;
        let c = Complex::new(t.cos(), 0.0);
        let s = Complex::new(0.0, -t.sin());
        let rx_op = operation!(
            [c, s;
             s, c])
        .expect("Cannot construct rx");
        self.sim
            .apply_operation(&rx_op, &[self.qman.map(q)])
            .expect("Noisy rx failed.");
    }
    fn rxx(&mut self, theta: f64, q0: usize, q1: usize) {
        let t = theta / 2.0;
        let c = Complex::new(t.cos(), 0.0);
        let s = Complex::new(0.0, -t.sin());
        let rxx_op = operation!(
            [c, 0.0, 0.0, s;
             0.0, c, s, 0.0;
             0.0, s, c, 0.0;
             s, 0.0, 0.0, c])
        .expect("Cannot construct rxx");
        self.sim
            .apply_operation(&rxx_op, &[self.qman.map(q1), self.qman.map(q0)])
            .expect("Noisy rxx failed.");
    }
    fn ry(&mut self, theta: f64, q: usize) {
        let t = theta / 2.0;
        let c = Complex::new(t.cos(), 0.0);
        let s = Complex::new(t.sin(), 0.0);
        let ry_op = operation!(
            [c, s.neg();
             s, c])
        .expect("Cannot construct ry");
        self.sim
            .apply_operation(&ry_op, &[self.qman.map(q)])
            .expect("Noisy ry failed.");
    }
    fn ryy(&mut self, theta: f64, q0: usize, q1: usize) {
        let t = theta / 2.0;
        let c = Complex::new(t.cos(), 0.0);
        let s = Complex::new(0.0, t.sin());
        let ryy_op = operation!(
            [c, 0.0, 0.0, s;
             0.0, c, s.neg(), 0.0;
             0.0, s.neg(), c, 0.0;
             s, 0.0, 0.0, c])
        .expect("Cannot construct ryy");
        self.sim
            .apply_operation(&ryy_op, &[self.qman.map(q1), self.qman.map(q0)])
            .expect("Noisy ryy failed.");
    }
    fn rz(&mut self, theta: f64, q: usize) {
        let t = theta / 2.0;
        let c = t.cos();
        let s = t.sin();
        let rz_op = operation!(
            [Complex::new(c, -s), 0.0;
             0.0, Complex::new(c, s)])
        .expect("Cannot construct rz");
        self.sim
            .apply_operation(&rz_op, &[self.qman.map(q)])
            .expect("Noisy rz failed.");
    }
    fn rzz(&mut self, theta: f64, q0: usize, q1: usize) {
        let t = theta / 2.0;
        let p = Complex::new(t.cos(), t.sin());
        let m = Complex::new(t.cos(), -t.sin());
        let rzz_op = operation!(
            [m, 0.0, 0.0, 0.0;
             0.0, p, 0.0, 0.0;
             0.0, 0.0, p, 0.0;
             0.0, 0.0, 0.0, m])
        .expect("Cannot construct rzz");
        self.sim
            .apply_operation(&rzz_op, &[self.qman.map(q1), self.qman.map(q0)])
            .expect("Noisy rzz failed.");
    }
    fn sadj(&mut self, q: usize) {
        self.sim
            .apply_operation(&self.sadj_op, &[self.qman.map(q)])
            .expect("Noisy sadj failed.");
    }
    fn s(&mut self, q: usize) {
        self.sim
            .apply_operation(&self.s_op, &[self.qman.map(q)])
            .expect("Noisy s failed.");
    }
    fn swap(&mut self, q0: usize, q1: usize) {
        self.sim
            .apply_operation(&self.swap_op, &[self.qman.map(q1), self.qman.map(q0)])
            .expect("Noisy swap failed.");
    }
    fn tadj(&mut self, q: usize) {
        self.sim
            .apply_operation(&self.tadj_op, &[self.qman.map(q)])
            .expect("Noisy tadj failed.");
    }
    fn t(&mut self, q: usize) {
        self.sim
            .apply_operation(&self.t_op, &[self.qman.map(q)])
            .expect("Noisy t failed.");
    }
    fn x(&mut self, q: usize) {
        self.sim
            .apply_operation(&self.x_op, &[self.qman.map(q)])
            .expect("Noisy x failed.");
    }
    fn y(&mut self, q: usize) {
        self.sim
            .apply_operation(&self.y_op, &[self.qman.map(q)])
            .expect("Noisy y failed.");
    }
    fn z(&mut self, q: usize) {
        self.sim
            .apply_operation(&self.z_op, &[self.qman.map(q)])
            .expect("Noisy z failed.");
    }
    fn qubit_allocate(&mut self) -> usize {
        let id = self.qman.allocate().expect("Out of qubits on allocate.");
        if id > self.max_qubit_id {
            self.max_qubit_id = id;
        }
        id
    }
    fn qubit_release(&mut self, q: usize) {
        self.qman.release(q);
    }
    fn qubit_swap_id(&mut self, q0: usize, q1: usize) {
        self.qman.swap(q0, q1);
    }
    fn capture_quantum_state(&mut self) -> (Vec<(BigUint, Complex<f64>)>, usize) {
        let state = self
            .sim
            .state()
            .expect("Cannot get state out of noisy simulator.")
            .data();
        let n = state.len();
        let mut result: Vec<(BigUint, Complex<f64>)> = Vec::with_capacity(n);
        for i in 0..n {
            let amplitude = state.get(i).unwrap();
            // TODO: Use state for tolerance
            if amplitude.norm() > 1e-12 {
                // Inverse bits of i for big-endianness!
                result.push((i.into(), *state.get(i).unwrap()));
            }
        }
        (result, self.max_qubit_id + 1)
    }
    fn qubit_is_zero(&mut self, _q: usize) -> bool {
        true // TODO: Perform actual check. Could be done by applying instrument without evolution.
    }
    fn custom_intrinsic(&mut self, _name: &str, _arg: Value) -> Option<Result<Value, String>> {
        unimplemented!("custom_intrinsic is not implemented.");
    }
    fn set_seed(&mut self, _seed: Option<u64>) {
        // TODO: Nothing. Noisy simulator doesn't support setting a seed.
    }
}

pub struct SparseNoisySim {
    pub sim: SparseSim,
    // Pauli noise probability distribution showing which Pauli gate to apply.
    // For a random value r drawn uniformly from [0, 1)
    // if r < prob_distr[0]: X gate is applied.
    // if prob_distr[0] <= r < prob_distr[1]: Y gate is applied.
    // if prob_distr[1] <= r < prob_distr[2]: Z gate is applied.
    // if prob_distr[2] <= r: I gate is applied (no-noise case).
    pub prob_distr: [f64; 3],
}

impl SparseNoisySim {
    #[must_use]
    pub fn new(_xyzi_probs: &[f64; 4]) -> Self {
        // TODO: Need to compute probability distribution from density
        Self {
            sim: SparseSim::new(),
            // TODO: This is a common noise. Need a way to provide per-gate noise.
            prob_distr: [0.001, 0.002, 0.003],
        }
    }
    pub fn apply_noise(&mut self, q: usize) {
        let r: f64 = rand::random();
        match r {
            x if x < self.prob_distr[0] => self.sim.x(q),
            x if x < self.prob_distr[1] => self.sim.y(q),
            x if x < self.prob_distr[2] => self.sim.z(q),
            _ => {} // I(q)
        }
    }
}

impl Backend for SparseNoisySim {
    type ResultType = bool;

    // TODO: Handle decompositions properly

    fn ccx(&mut self, ctl0: usize, ctl1: usize, q: usize) {
        self.sim.ccx(ctl0, ctl1, q);
        self.apply_noise(ctl0);
        self.apply_noise(ctl1);
        self.apply_noise(q);
    }

    fn cx(&mut self, ctl: usize, q: usize) {
        self.sim.cx(ctl, q);
        self.apply_noise(ctl);
        self.apply_noise(q);
    }

    fn cy(&mut self, ctl: usize, q: usize) {
        self.sim.cy(ctl, q);
        self.apply_noise(ctl);
        self.apply_noise(q);
    }

    fn cz(&mut self, ctl: usize, q: usize) {
        self.sim.cz(ctl, q);
        self.apply_noise(ctl);
        self.apply_noise(q);
    }

    fn h(&mut self, q: usize) {
        self.sim.h(q);
        self.apply_noise(q);
    }

    fn m(&mut self, q: usize) -> Self::ResultType {
        // TODO: Handle Measurement
        let result = self.sim.m(q);
        self.apply_noise(q);
        result
    }

    fn mresetz(&mut self, q: usize) -> Self::ResultType {
        // TODO: Handle Measurement
        let result = self.sim.mresetz(q);
        self.apply_noise(q);
        result
    }

    fn reset(&mut self, q: usize) {
        self.sim.reset(q);
        self.apply_noise(q);
    }

    fn rx(&mut self, theta: f64, q: usize) {
        self.sim.rx(theta, q);
        self.apply_noise(q);
    }

    fn rxx(&mut self, theta: f64, q0: usize, q1: usize) {
        self.sim.rxx(theta, q0, q1);
        self.apply_noise(q0);
        self.apply_noise(q1);
    }

    fn ry(&mut self, theta: f64, q: usize) {
        self.sim.ry(theta, q);
        self.apply_noise(q);
    }

    fn ryy(&mut self, theta: f64, q0: usize, q1: usize) {
        self.sim.ryy(theta, q0, q1);
        self.apply_noise(q0);
        self.apply_noise(q1);
    }

    fn rz(&mut self, theta: f64, q: usize) {
        self.sim.rz(theta, q);
        self.apply_noise(q);
    }

    fn rzz(&mut self, theta: f64, q0: usize, q1: usize) {
        self.sim.rzz(theta, q0, q1);
        self.apply_noise(q0);
        self.apply_noise(q1);
    }

    fn sadj(&mut self, q: usize) {
        self.sim.sadj(q);
        self.apply_noise(q);
    }

    fn s(&mut self, q: usize) {
        self.sim.s(q);
        self.apply_noise(q);
    }

    fn swap(&mut self, q0: usize, q1: usize) {
        self.sim.swap(q0, q1);
        self.apply_noise(q0);
        self.apply_noise(q1);
    }

    fn tadj(&mut self, q: usize) {
        self.sim.tadj(q);
        self.apply_noise(q);
    }

    fn t(&mut self, q: usize) {
        self.sim.t(q);
        self.apply_noise(q);
    }

    fn x(&mut self, q: usize) {
        self.sim.x(q);
        self.apply_noise(q);
    }

    fn y(&mut self, q: usize) {
        self.sim.y(q);
        self.apply_noise(q);
    }

    fn z(&mut self, q: usize) {
        self.sim.z(q);
        self.apply_noise(q);
    }

    fn qubit_allocate(&mut self) -> usize {
        let q: usize = self.sim.qubit_allocate();
        self.apply_noise(q);
        q
    }

    fn qubit_release(&mut self, q: usize) {
        // TODO: Does this throw if qubit is in a non-zero state?
        // With noise it may be problematic to put qubit in a zero state.
        self.sim.reset(q);
        self.sim.qubit_release(q);
    }

    fn qubit_swap_id(&mut self, q0: usize, q1: usize) {
        self.sim.qubit_swap_id(q0, q1);
        // TODO: This is probably noiseless operation as it doesn't apply any gates.
    }

    fn capture_quantum_state(
        &mut self,
    ) -> (Vec<(num_bigint::BigUint, num_complex::Complex<f64>)>, usize) {
        self.sim.capture_quantum_state()
    }

    fn qubit_is_zero(&mut self, q: usize) -> bool {
        self.sim.qubit_is_zero(q)
    }

    fn custom_intrinsic(&mut self, name: &str, arg: Value) -> Option<Result<Value, String>> {
        self.sim.custom_intrinsic(name, arg)
        // TODO: Should this be noisy? If so, do we know which qubits it affects?
    }

    fn set_seed(&mut self, seed: Option<u64>) {
        self.sim.set_seed(seed);
        // TODO: Should this also be a seed for noisy rng?
    }
}

/// Simple struct that chains two backends together so that the chained
/// backend is called before the main backend.
/// For any intrinsics that return a value,
/// the value returned by the chained backend is ignored.
/// The value returned by the main backend is returned.
pub struct Chain<T1, T2> {
    pub main: T1,
    pub chained: T2,
}

impl<T1, T2> Chain<T1, T2>
where
    T1: Backend,
    T2: Backend,
{
    pub fn new(primary: T1, chained: T2) -> Chain<T1, T2> {
        Chain {
            main: primary,
            chained,
        }
    }
}

impl<T1, T2> Backend for Chain<T1, T2>
where
    T1: Backend,
    T2: Backend,
{
    type ResultType = T1::ResultType;

    fn ccx(&mut self, ctl0: usize, ctl1: usize, q: usize) {
        self.chained.ccx(ctl0, ctl1, q);
        self.main.ccx(ctl0, ctl1, q);
    }

    fn cx(&mut self, ctl: usize, q: usize) {
        self.chained.cx(ctl, q);
        self.main.cx(ctl, q);
    }

    fn cy(&mut self, ctl: usize, q: usize) {
        self.chained.cy(ctl, q);
        self.main.cy(ctl, q);
    }

    fn cz(&mut self, ctl: usize, q: usize) {
        self.chained.cz(ctl, q);
        self.main.cz(ctl, q);
    }

    fn h(&mut self, q: usize) {
        self.chained.h(q);
        self.main.h(q);
    }

    fn m(&mut self, q: usize) -> Self::ResultType {
        let _ = self.chained.m(q);
        self.main.m(q)
    }

    fn mresetz(&mut self, q: usize) -> Self::ResultType {
        let _ = self.chained.mresetz(q);
        self.main.mresetz(q)
    }

    fn reset(&mut self, q: usize) {
        self.chained.reset(q);
        self.main.reset(q);
    }

    fn rx(&mut self, theta: f64, q: usize) {
        self.chained.rx(theta, q);
        self.main.rx(theta, q);
    }

    fn rxx(&mut self, theta: f64, q0: usize, q1: usize) {
        self.chained.rxx(theta, q0, q1);
        self.main.rxx(theta, q0, q1);
    }

    fn ry(&mut self, theta: f64, q: usize) {
        self.chained.ry(theta, q);
        self.main.ry(theta, q);
    }

    fn ryy(&mut self, theta: f64, q0: usize, q1: usize) {
        self.chained.ryy(theta, q0, q1);
        self.main.ryy(theta, q0, q1);
    }

    fn rz(&mut self, theta: f64, q: usize) {
        self.chained.rz(theta, q);
        self.main.rz(theta, q);
    }

    fn rzz(&mut self, theta: f64, q0: usize, q1: usize) {
        self.chained.rzz(theta, q0, q1);
        self.main.rzz(theta, q0, q1);
    }

    fn sadj(&mut self, q: usize) {
        self.chained.sadj(q);
        self.main.sadj(q);
    }

    fn s(&mut self, q: usize) {
        self.chained.s(q);
        self.main.s(q);
    }

    fn swap(&mut self, q0: usize, q1: usize) {
        self.chained.swap(q0, q1);
        self.main.swap(q0, q1);
    }

    fn tadj(&mut self, q: usize) {
        self.chained.tadj(q);
        self.main.tadj(q);
    }

    fn t(&mut self, q: usize) {
        self.chained.t(q);
        self.main.t(q);
    }

    fn x(&mut self, q: usize) {
        self.chained.x(q);
        self.main.x(q);
    }

    fn y(&mut self, q: usize) {
        self.chained.y(q);
        self.main.y(q);
    }

    fn z(&mut self, q: usize) {
        self.chained.z(q);
        self.main.z(q);
    }

    fn qubit_allocate(&mut self) -> usize {
        // Warning: we use the qubit id allocated by the
        // main backend, even for later calls into the chained
        // backend. This is not an issue today, but could
        // become an issue if the qubit ids differ between
        // the two backends.
        let _ = self.chained.qubit_allocate();
        self.main.qubit_allocate()
    }

    fn qubit_release(&mut self, q: usize) {
        self.chained.qubit_release(q);
        self.main.qubit_release(q);
    }

    fn qubit_swap_id(&mut self, q0: usize, q1: usize) {
        self.chained.qubit_swap_id(q0, q1);
        self.main.qubit_swap_id(q0, q1);
    }

    fn capture_quantum_state(
        &mut self,
    ) -> (Vec<(num_bigint::BigUint, num_complex::Complex<f64>)>, usize) {
        let _ = self.chained.capture_quantum_state();
        self.main.capture_quantum_state()
    }

    fn qubit_is_zero(&mut self, q: usize) -> bool {
        let _ = self.chained.qubit_is_zero(q);
        self.main.qubit_is_zero(q)
    }

    fn custom_intrinsic(&mut self, name: &str, arg: Value) -> Option<Result<Value, String>> {
        let _ = self.chained.custom_intrinsic(name, arg.clone());
        self.main.custom_intrinsic(name, arg)
    }

    fn set_seed(&mut self, seed: Option<u64>) {
        self.chained.set_seed(seed);
        self.main.set_seed(seed);
    }
}

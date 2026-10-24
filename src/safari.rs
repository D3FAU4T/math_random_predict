use crate::solver::{BV, Solver, SatResult};

pub struct SafariPredictor {
    sequence: Vec<f64>,
    conc_state_0: u64,
    conc_state_1: u64,
    is_solved: bool,
}

impl SafariPredictor {
    pub fn new(sequence: Vec<f64>) -> Self {
        Self {
            sequence,
            conc_state_0: 0,
            conc_state_1: 0,
            is_solved: false,
        }
    }

    fn xor_shift_128_plus_concrete(&mut self) -> u64 {
        let s1 = self.conc_state_0;
        let s0 = self.conc_state_1;
        self.conc_state_0 = s0;
        let mut s1_new = s1 ^ (s1 << 23);
        s1_new = s1_new ^ (s1_new >> 17);
        s1_new = s1_new ^ s0 ^ (s0 >> 26);
        self.conc_state_1 = s1_new;
        self.conc_state_1.wrapping_add(s0)
    }

    fn to_double(&self, value: u64) -> f64 {
        ((value & 0x1FFFFFFFFFFFFF) as f64) / ((1u64 << 53) as f64)
    }

    pub fn solve(&mut self) -> bool {
        let solver = Solver::new();

        let mut curr_state_0 = BV::new_const("sym_state_0", 64);
        let mut curr_state_1 = BV::new_const("sym_state_1", 64);

        for &observed in &self.sequence {
            let s1 = curr_state_0.clone();
            let s0 = curr_state_1.clone();

            // s1 ^ (s1 << 23)
            let mut s1_next = s1.bvxor(&s1.bvshl(&BV::from_u64(23, 64)));
            // s1_next ^ (s1_next >> 17)
            s1_next = s1_next.bvxor(&s1_next.bvlshr(&BV::from_u64(17, 64)));
            // s1_next ^ s0 ^ (s0 >> 26)
            s1_next = s1_next.bvxor(&s0).bvxor(&s0.bvlshr(&BV::from_u64(26, 64)));

            curr_state_0 = s0;
            curr_state_1 = s1_next;

            let mantissa = (observed * (1u64 << 53) as f64).round() as u64;
            let sum_val = curr_state_0.bvadd(&curr_state_1);
            let mask = BV::from_u64(0x1FFFFFFFFFFFFF, 64);
            let constrained_val = sum_val.bvand(&mask);

            solver.assert(&constrained_val.eq(BV::from_u64(mantissa, 64)));
        }

        if solver.check() == SatResult::Sat {
            if let Some(model) = solver.get_model() {
                self.is_solved = true;
                self.conc_state_0 = model.eval(&curr_state_0, true).unwrap().as_u64().unwrap();
                self.conc_state_1 = model.eval(&curr_state_1, true).unwrap().as_u64().unwrap();
                return true;
            }
        }

        false
    }

    pub fn predict_next(&mut self) -> f64 {
        assert!(self.is_solved, "State not solved yet!");
        let v = self.xor_shift_128_plus_concrete();
        self.to_double(v)
    }
}

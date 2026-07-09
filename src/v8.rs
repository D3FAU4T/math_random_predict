use z3::ast::BV;
use z3::Solver;

pub enum V8Mode {
    Modern,
    V24,
    Legacy,
}

pub struct V8Predictor {
    internal_sequence: Vec<f64>,
    mode: V8Mode,
    conc_state_0: u64,
    conc_state_1: u64,
    is_solved: bool,
}

impl V8Predictor {
    pub fn new(sequence: Vec<f64>, mode: V8Mode) -> Self {
        let mut internal_sequence = sequence.clone();
        internal_sequence.reverse();
        Self {
            internal_sequence,
            mode,
            conc_state_0: 0,
            conc_state_1: 0,
            is_solved: false,
        }
    }

    fn xor_shift_128_plus_concrete(&mut self) -> u64 {
        let t1 = self.conc_state_0;
        let mut t0 = self.conc_state_1 ^ (self.conc_state_0 >> 26);
        t0 ^= self.conc_state_0;
        t0 ^= (t0 >> 17) ^ (t0 >> 34) ^ (t0 >> 51);
        t0 ^= (t0 << 23) ^ (t0 << 46);
        
        self.conc_state_0 = t0;
        self.conc_state_1 = t1;
        
        match self.mode {
            V8Mode::Modern => self.conc_state_0.wrapping_add(self.conc_state_1),
            _ => t1,
        }
    }

    fn to_double(&self, value: u64) -> f64 {
        match self.mode {
            V8Mode::Modern | V8Mode::V24 => {
                ((value >> 11) as f64) / ((1u64 << 53) as f64)
            }
            V8Mode::Legacy => {
                let u64_val = (value >> 12) | 0x3FF0000000000000;
                f64::from_bits(u64_val) - 1.0
            }
        }
    }

    pub fn solve(&mut self) -> bool {
        let solver = Solver::new();

        let mut curr_state_0 = BV::new_const("sym_state_0", 64);
        let mut curr_state_1 = BV::new_const("sym_state_1", 64);

        for &observed in &self.internal_sequence {
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

            match self.mode {
                V8Mode::Modern => {
                    let mantissa = (observed * (1u64 << 53) as f64).round() as u64;
                    let sum_val = curr_state_0.bvadd(&curr_state_1);
                    let constrained_val = sum_val.bvlshr(&BV::from_u64(11, 64));
                    solver.assert(&constrained_val.eq(BV::from_u64(mantissa, 64)));
                }
                V8Mode::V24 => {
                    let mantissa = (observed * (1u64 << 53) as f64).round() as u64;
                    let constrained_val = curr_state_0.bvlshr(&BV::from_u64(11, 64));
                    solver.assert(&constrained_val.eq(BV::from_u64(mantissa, 64)));
                }
                V8Mode::Legacy => {
                    let u64_bits = (observed + 1.0).to_bits();
                    let mantissa = u64_bits & ((1u64 << 52) - 1);
                    let constrained_val = curr_state_0.bvlshr(&BV::from_u64(12, 64));
                    solver.assert(&constrained_val.eq(BV::from_u64(mantissa, 64)));
                }
            }
        }

        if solver.check() == z3::SatResult::Sat {
            if let Some(model) = solver.get_model() {
                self.conc_state_0 = model.eval(&curr_state_0, true).unwrap().as_u64().unwrap();
                self.conc_state_1 = model.eval(&curr_state_1, true).unwrap().as_u64().unwrap();
                
                for _ in 0..(self.internal_sequence.len() - 1) {
                    self.xor_shift_128_plus_concrete();
                }

                self.is_solved = true;
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

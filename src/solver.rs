use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ----------------- CDCL SAT Solver -----------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lit(i32);

impl Lit {
    pub fn new(var: u32, sign: bool) -> Self {
        let v = var as i32;
        Lit(if sign { -v } else { v })
    }
    pub fn var(self) -> usize {
        self.0.unsigned_abs() as usize
    }
    pub fn sign(self) -> bool {
        self.0 < 0
    }
    pub fn not(self) -> Self {
        Lit(-self.0)
    }
    pub fn index(self) -> usize {
        let v = self.var();
        if self.sign() { (v << 1) | 1 } else { v << 1 }
    }
}

pub struct SatSolver {
    pub num_vars: usize,
    pub clauses: Vec<Vec<Lit>>,
    pub watches: Vec<Vec<usize>>,
    pub assigns: Vec<i8>, // 0: undef, 1: true, -1: false
    pub trail: Vec<Lit>,
    pub trail_lim: Vec<usize>,
    pub reason: Vec<Option<usize>>,
    pub level: Vec<u32>,
    pub activity: Vec<f64>,
    pub var_inc: f64,
    pub seen: Vec<bool>,
    pub true_var: usize,
}

impl SatSolver {
    pub fn new() -> Self {
        let mut s = Self {
            num_vars: 0,
            clauses: Vec::new(),
            watches: Vec::new(),
            assigns: Vec::new(),
            trail: Vec::new(),
            trail_lim: Vec::new(),
            reason: Vec::new(),
            level: Vec::new(),
            activity: Vec::new(),
            var_inc: 1.0,
            seen: Vec::new(),
            true_var: 0,
        };
        s.new_var(); // var 0 dummy
        s.true_var = s.new_var(); // var 1 = TRUE
        s.assign(Lit::new(s.true_var as u32, false), None);
        s
    }

    pub fn true_lit(&self) -> Lit {
        Lit::new(self.true_var as u32, false)
    }

    pub fn false_lit(&self) -> Lit {
        Lit::new(self.true_var as u32, true)
    }

    pub fn new_var(&mut self) -> usize {
        self.num_vars += 1;
        let v = self.num_vars - 1;
        self.assigns.push(0);
        self.reason.push(None);
        self.level.push(0);
        self.activity.push(0.0);
        self.seen.push(false);
        self.watches.resize((v + 1) * 2 + 2, Vec::new());
        v
    }

    pub fn add_clause(&mut self, lits: Vec<Lit>) -> bool {
        let mut simplified = Vec::new();
        for &lit in &lits {
            if lit == self.true_lit() {
                return true;
            }
            if lit == self.false_lit() {
                continue;
            }
            if !simplified.contains(&lit) {
                simplified.push(lit);
            }
        }
        if simplified.is_empty() {
            return false;
        }
        if simplified.len() == 1 {
            return self.assign(simplified[0], None);
        }
        let cr = self.clauses.len();
        self.watches[simplified[0].not().index()].push(cr);
        self.watches[simplified[1].not().index()].push(cr);
        self.clauses.push(simplified);
        true
    }

    fn decision_level(&self) -> u32 {
        self.trail_lim.len() as u32
    }

    pub fn assign(&mut self, lit: Lit, reason: Option<usize>) -> bool {
        let var = lit.var();
        let val = if lit.sign() { -1 } else { 1 };
        if self.assigns[var] != 0 {
            return self.assigns[var] == val;
        }
        self.assigns[var] = val;
        self.reason[var] = reason;
        self.level[var] = self.decision_level();
        self.trail.push(lit);
        true
    }

    pub fn propagate(&mut self) -> Option<usize> {
        let mut i = 0;
        while i < self.trail.len() {
            let p = self.trail[i];
            i += 1;
            let p_idx = p.index();
            let mut j = 0;
            while j < self.watches[p_idx].len() {
                let cr = self.watches[p_idx][j];
                let clause = &mut self.clauses[cr];

                if clause[0] == p.not() {
                    clause.swap(0, 1);
                }

                let first = clause[0];
                let first_val = self.assigns[first.var()] * if first.sign() { -1 } else { 1 };
                if first_val == 1 {
                    j += 1;
                    continue;
                }

                let mut found = false;
                for k in 2..clause.len() {
                    let lit = clause[k];
                    let val = self.assigns[lit.var()] * if lit.sign() { -1 } else { 1 };
                    if val != -1 {
                        clause.swap(1, k);
                        let new_watch = clause[1].not().index();
                        self.watches[new_watch].push(cr);
                        self.watches[p_idx].swap_remove(j);
                        found = true;
                        break;
                    }
                }

                if !found {
                    j += 1;
                    if !self.assign(first, Some(cr)) {
                        return Some(cr);
                    }
                }
            }
        }
        None
    }

    fn analyze(&mut self, mut confl: usize) -> (Vec<Lit>, u32) {
        let mut learnt = vec![Lit::new(0, false)];
        let mut path_cnt = 0;
        let mut p: Option<Lit> = None;
        let mut trail_idx = self.trail.len();
        let cur_level = self.decision_level();

        loop {
            let clause = &self.clauses[confl];
            let start = if p.is_none() { 0 } else { 1 };
            for k in start..clause.len() {
                let lit = clause[k];
                let var = lit.var();
                if !self.seen[var] && self.level[var] > 0 {
                    self.seen[var] = true;
                    self.activity[var] += self.var_inc;
                    if self.level[var] >= cur_level {
                        path_cnt += 1;
                    } else {
                        learnt.push(lit);
                    }
                }
            }

            while trail_idx > 0 {
                trail_idx -= 1;
                let next_lit = self.trail[trail_idx];
                if self.seen[next_lit.var()] {
                    p = Some(next_lit);
                    break;
                }
            }

            let lit = p.unwrap();
            self.seen[lit.var()] = false;
            path_cnt -= 1;
            if path_cnt <= 0 {
                learnt[0] = lit.not();
                break;
            }
            confl = self.reason[lit.var()].unwrap();
        }

        for &lit in &learnt {
            self.seen[lit.var()] = false;
        }

        let mut max_level = 0;
        for i in 1..learnt.len() {
            let lvl = self.level[learnt[i].var()];
            if lvl > max_level {
                max_level = lvl;
                learnt.swap(1, i);
            }
        }

        self.var_inc *= 1.05;
        (learnt, max_level)
    }

    pub fn backtrack(&mut self, level: u32) {
        if self.decision_level() > level {
            let start = self.trail_lim[level as usize];
            for k in (start..self.trail.len()).rev() {
                let var = self.trail[k].var();
                self.assigns[var] = 0;
                self.reason[var] = None;
            }
            self.trail.truncate(start);
            self.trail_lim.truncate(level as usize);
        }
    }

    pub fn solve(&mut self) -> bool {
        if self.propagate().is_some() {
            return false;
        }

        loop {
            let mut best_var = 0;
            let mut max_act = -1.0;
            for v in 1..=self.num_vars {
                if v < self.assigns.len() && self.assigns[v] == 0 {
                    if self.activity[v] > max_act {
                        max_act = self.activity[v];
                        best_var = v;
                    }
                }
            }

            if best_var == 0 {
                return true;
            }

            self.trail_lim.push(self.trail.len());
            let dec_lit = Lit::new(best_var as u32, false);
            self.assign(dec_lit, None);

            while let Some(confl) = self.propagate() {
                if self.decision_level() == 0 {
                    return false;
                }

                let (learnt, back_level) = self.analyze(confl);
                self.backtrack(back_level);

                if learnt.len() == 1 {
                    self.assign(learnt[0], None);
                } else {
                    let cr = self.clauses.len();
                    self.watches[learnt[0].not().index()].push(cr);
                    self.watches[learnt[1].not().index()].push(cr);
                    self.clauses.push(learnt.clone());
                    self.assign(learnt[0], Some(cr));
                }
            }
        }
    }

    pub fn val(&self, lit: Lit) -> bool {
        let v = lit.var();
        let val = self.assigns[v] == 1;
        if lit.sign() { !val } else { val }
    }
}

// ----------------- BitVector AST & Solver API -----------------

static VAR_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum BVExpr {
    Var(usize),
    Const(u64),
    Xor(Rc<BVExpr>, Rc<BVExpr>),
    Shl(Rc<BVExpr>, usize),
    Lshr(Rc<BVExpr>, usize),
    And(Rc<BVExpr>, Rc<BVExpr>),
    Add(Rc<BVExpr>, Rc<BVExpr>),
}

#[derive(Clone, Debug)]
pub struct BV {
    pub expr: Rc<BVExpr>,
    pub width: usize,
}

#[derive(Clone, Debug)]
pub struct BoolExpr {
    pub left: BV,
    pub right: BV,
}

impl BV {
    pub fn new_const(_name: &str, width: usize) -> Self {
        let id = VAR_COUNTER.fetch_add(1, Ordering::Relaxed);
        BV {
            expr: Rc::new(BVExpr::Var(id)),
            width,
        }
    }

    pub fn from_u64(val: u64, width: usize) -> Self {
        BV {
            expr: Rc::new(BVExpr::Const(val)),
            width,
        }
    }

    pub fn bvxor(&self, other: &Self) -> Self {
        BV {
            expr: Rc::new(BVExpr::Xor(Rc::clone(&self.expr), Rc::clone(&other.expr))),
            width: self.width,
        }
    }

    pub fn bvand(&self, other: &Self) -> Self {
        BV {
            expr: Rc::new(BVExpr::And(Rc::clone(&self.expr), Rc::clone(&other.expr))),
            width: self.width,
        }
    }

    pub fn bvadd(&self, other: &Self) -> Self {
        BV {
            expr: Rc::new(BVExpr::Add(Rc::clone(&self.expr), Rc::clone(&other.expr))),
            width: self.width,
        }
    }

    pub fn bvshl(&self, shift_bv: &Self) -> Self {
        let shift = match &*shift_bv.expr {
            BVExpr::Const(c) => *c as usize,
            _ => panic!("Non-constant shift"),
        };
        BV {
            expr: Rc::new(BVExpr::Shl(Rc::clone(&self.expr), shift)),
            width: self.width,
        }
    }

    pub fn bvlshr(&self, shift_bv: &Self) -> Self {
        let shift = match &*shift_bv.expr {
            BVExpr::Const(c) => *c as usize,
            _ => panic!("Non-constant shift"),
        };
        BV {
            expr: Rc::new(BVExpr::Lshr(Rc::clone(&self.expr), shift)),
            width: self.width,
        }
    }

    pub fn eq(&self, other: BV) -> BoolExpr {
        BoolExpr {
            left: self.clone(),
            right: other,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SatResult {
    Sat,
    Unsat,
}

pub struct ModelVal {
    val: u64,
}

impl ModelVal {
    pub fn as_u64(&self) -> Option<u64> {
        Some(self.val)
    }
}

#[derive(Clone)]
pub struct Model {
    node_values: HashMap<usize, u64>,
}

impl Model {
    pub fn eval(&self, bv: &BV, _model_completion: bool) -> Option<ModelVal> {
        let key = Rc::as_ptr(&bv.expr) as usize;
        self.node_values.get(&key).map(|&val| ModelVal { val })
    }
}

pub struct Solver {
    assertions: RefCell<Vec<BoolExpr>>,
    model: RefCell<Option<Model>>,
}

impl Solver {
    pub fn new() -> Self {
        Self {
            assertions: RefCell::new(Vec::new()),
            model: RefCell::new(None),
        }
    }

    pub fn assert(&self, expr: &BoolExpr) {
        self.assertions.borrow_mut().push(expr.clone());
    }

    pub fn check(&self) -> SatResult {
        let mut sat = SatSolver::new();
        let mut memo: HashMap<usize, Vec<Lit>> = HashMap::new();

        fn blast(
            expr: &Rc<BVExpr>,
            width: usize,
            sat: &mut SatSolver,
            memo: &mut HashMap<usize, Vec<Lit>>,
        ) -> Vec<Lit> {
            let key = Rc::as_ptr(expr) as usize;
            if let Some(bits) = memo.get(&key) {
                return bits.clone();
            }

            let bits = match &**expr {
                BVExpr::Const(val) => {
                    let mut b = Vec::with_capacity(width);
                    for i in 0..width {
                        let bit = ((val >> i) & 1) == 1;
                        b.push(if bit { sat.true_lit() } else { sat.false_lit() });
                    }
                    b
                }
                BVExpr::Var(_) => {
                    let mut b = Vec::with_capacity(width);
                    for _ in 0..width {
                        let v = sat.new_var();
                        sat.activity[v] = 100.0;
                        b.push(Lit::new(v as u32, false));
                    }
                    b
                }
                BVExpr::Xor(l, r) => {
                    let lb = blast(l, width, sat, memo);
                    let rb = blast(r, width, sat, memo);
                    let mut b = Vec::with_capacity(width);
                    for i in 0..width {
                        let x = lb[i];
                        let y = rb[i];
                        if x == sat.false_lit() {
                            b.push(y);
                        } else if y == sat.false_lit() {
                            b.push(x);
                        } else if x == sat.true_lit() {
                            b.push(y.not());
                        } else if y == sat.true_lit() {
                            b.push(x.not());
                        } else if x == y {
                            b.push(sat.false_lit());
                        } else if x == y.not() {
                            b.push(sat.true_lit());
                        } else {
                            let out = Lit::new(sat.new_var() as u32, false);
                            sat.add_clause(vec![x.not(), y.not(), out.not()]);
                            sat.add_clause(vec![x, y, out.not()]);
                            sat.add_clause(vec![x, y.not(), out]);
                            sat.add_clause(vec![x.not(), y, out]);
                            b.push(out);
                        }
                    }
                    b
                }
                BVExpr::And(l, r) => {
                    let lb = blast(l, width, sat, memo);
                    let rb = blast(r, width, sat, memo);
                    let mut b = Vec::with_capacity(width);
                    for i in 0..width {
                        let x = lb[i];
                        let y = rb[i];
                        if x == sat.false_lit() || y == sat.false_lit() {
                            b.push(sat.false_lit());
                        } else if x == sat.true_lit() {
                            b.push(y);
                        } else if y == sat.true_lit() {
                            b.push(x);
                        } else if x == y {
                            b.push(x);
                        } else {
                            let out = Lit::new(sat.new_var() as u32, false);
                            sat.add_clause(vec![x.not(), y.not(), out]);
                            sat.add_clause(vec![x, out.not()]);
                            sat.add_clause(vec![y, out.not()]);
                            b.push(out);
                        }
                    }
                    b
                }
                BVExpr::Shl(inner, shift) => {
                    let ib = blast(inner, width, sat, memo);
                    let mut b = Vec::with_capacity(width);
                    for i in 0..width {
                        if i < *shift {
                            b.push(sat.false_lit());
                        } else {
                            b.push(ib[i - shift]);
                        }
                    }
                    b
                }
                BVExpr::Lshr(inner, shift) => {
                    let ib = blast(inner, width, sat, memo);
                    let mut b = Vec::with_capacity(width);
                    for i in 0..width {
                        if i + shift < width {
                            b.push(ib[i + shift]);
                        } else {
                            b.push(sat.false_lit());
                        }
                    }
                    b
                }
                BVExpr::Add(l, r) => {
                    let lb = blast(l, width, sat, memo);
                    let rb = blast(r, width, sat, memo);
                    let mut b = Vec::with_capacity(width);
                    let mut carry = sat.false_lit();

                    for i in 0..width {
                        let x = lb[i];
                        let y = rb[i];
                        let c = carry;

                        let xy_xor = {
                            let out = Lit::new(sat.new_var() as u32, false);
                            sat.add_clause(vec![x.not(), y.not(), out.not()]);
                            sat.add_clause(vec![x, y, out.not()]);
                            sat.add_clause(vec![x, y.not(), out]);
                            sat.add_clause(vec![x.not(), y, out]);
                            out
                        };

                        let sum = {
                            let out = Lit::new(sat.new_var() as u32, false);
                            sat.add_clause(vec![xy_xor.not(), c.not(), out.not()]);
                            sat.add_clause(vec![xy_xor, c, out.not()]);
                            sat.add_clause(vec![xy_xor, c.not(), out]);
                            sat.add_clause(vec![xy_xor.not(), c, out]);
                            out
                        };
                        b.push(sum);

                        let cout = {
                            let out = Lit::new(sat.new_var() as u32, false);
                            sat.add_clause(vec![x.not(), y.not(), out]);
                            sat.add_clause(vec![x.not(), c.not(), out]);
                            sat.add_clause(vec![y.not(), c.not(), out]);
                            sat.add_clause(vec![x, y, out.not()]);
                            sat.add_clause(vec![x, c, out.not()]);
                            sat.add_clause(vec![y, c, out.not()]);
                            out
                        };
                        carry = cout;
                    }
                    b
                }
            };

            memo.insert(key, bits.clone());
            bits
        }

        for assertion in self.assertions.borrow().iter() {
            let left_bits = blast(&assertion.left.expr, assertion.left.width, &mut sat, &mut memo);
            let right_bits = blast(&assertion.right.expr, assertion.right.width, &mut sat, &mut memo);
            assert_eq!(left_bits.len(), right_bits.len());
            for i in 0..left_bits.len() {
                let a = left_bits[i];
                let b = right_bits[i];
                sat.add_clause(vec![a.not(), b]);
                sat.add_clause(vec![a, b.not()]);
            }
        }

        if sat.solve() {
            let mut node_values = HashMap::new();
            for (&key, bits) in &memo {
                let mut val = 0u64;
                for (i, &bit) in bits.iter().enumerate() {
                    if sat.val(bit) {
                        val |= 1u64 << i;
                    }
                }
                node_values.insert(key, val);
            }
            *self.model.borrow_mut() = Some(Model { node_values });
            SatResult::Sat
        } else {
            SatResult::Unsat
        }
    }

    pub fn get_model(&self) -> Option<Model> {
        self.model.borrow().clone()
    }
}

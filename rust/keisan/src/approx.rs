pub trait ApproxEq<Rhs = Self> {
    fn approx_eq(&self, other: Rhs) -> bool;
    fn approx_eq_with_tolerance(&self, other: Rhs, tolerance: f64) -> bool;
}

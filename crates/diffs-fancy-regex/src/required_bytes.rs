use crate::{Expr, LookAround};

pub(crate) fn required(expr: &Expr) -> Option<u128> {
    match expr {
        Expr::Literal { val, casei: false } => Some(
            val.bytes()
                .filter(u8::is_ascii)
                .fold(0, |bytes, byte| bytes | (1u128 << byte)),
        ),
        Expr::Concat(children) => children
            .iter()
            .try_fold(0, |bytes, child| Some(bytes | required(child)?)),
        Expr::Alt(children) => children
            .iter()
            .try_fold(u128::MAX, |bytes, child| Some(bytes & required(child)?)),
        Expr::Group(child) | Expr::AtomicGroup(child) => required(child),
        Expr::LookAround(child, LookAround::LookAhead | LookAround::LookBehind) => required(child),
        Expr::Repeat { child, lo, .. } => {
            required(child).map(|bytes| if *lo == 0 { 0 } else { bytes })
        }
        Expr::Conditional { .. }
        | Expr::SubroutineCall(_)
        | Expr::UnresolvedNamedSubroutineCall { .. }
        | Expr::BackrefWithRelativeRecursionLevel { .. } => None,
        Expr::LookAround(child, _) => required(child).map(|_| 0),
        _ => Some(0),
    }
}

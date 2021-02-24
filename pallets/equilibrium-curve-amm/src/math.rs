use substrate_fixed::traits::{Fixed, FixedSigned, ToFixed};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct MathContext;

impl MathContext {
    pub fn new() -> MathContext {
        MathContext{}
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum MathError {
    Overflow,
    DivisionByZero,
}

pub type MathResult<T> = Result<T, (MathContext, MathError)>;

fn add<F: Fixed>(ctx: MathContext, a: F, b: F) -> MathResult<F> {
    a.checked_add(b).ok_or((ctx, MathError::Overflow))
}

fn sub<F: Fixed>(ctx: MathContext, a: F, b: F) -> MathResult<F> {
    a.checked_sub(b).ok_or((ctx, MathError::Overflow))
}

fn mul<F: Fixed>(ctx: MathContext, a: F, b: F) -> MathResult<F> {
    a.checked_mul(b).ok_or((ctx, MathError::Overflow))
}

fn div<F: Fixed>(ctx: MathContext, a: F, b: F) -> MathResult<F> {
    a.checked_div(b).ok_or((ctx, MathError::DivisionByZero))
}


fn get_d<F: Fixed>(ctx: MathContext, xp: Vec<F>, amp: F) -> MathResult<F> {
    let mut s: F = F::from_num(0);
    let mut d_prev: F = F::from_num(0);

    let n_coins = F::from_num(xp.len());

    for &x in xp.iter() {
        s = add(ctx, s, x)?;
    }
    if s == F::from_num(0) {
        return Ok(F::from_num(0));
    }

    let mut d: F = s;
    let ann: F = mul(ctx, amp, n_coins)?;

    for i in 0..255 {
        let mut d_p: F = d;
        for &x in xp.iter() {
            d_p = div(ctx, mul(ctx, d_p, d)?, mul(ctx, x, n_coins)?)?;
        }
        d_prev = d;
        d = div(ctx, 
                mul(ctx, add(ctx, mul(ctx, ann, s)?, mul(ctx, d_p, n_coins)?)?, d)?,
                mul(ctx,
                    add(ctx,
                        add(ctx, 
                             mul(ctx, sub(ctx, ann, F::from_num(1))?, d)?,
                             n_coins
                         )?, F::from_num(1)
                    )?,
                    d_p
                )?
            )?;
    }

    Ok(F::from_num(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    type FixedNumber = substrate_fixed::types::I64F64;

    #[test]
    fn test_add() {
        assert_eq!(
            add(MathContext::new(), FixedNumber::from_num(1), FixedNumber::from_num(2)),
            Ok(FixedNumber::from_num(3))
        );
    }
}

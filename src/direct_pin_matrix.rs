//! Hardware pin switch handling for switches directly attached to pins.

use embedded_hal::digital::v2::InputPin;

/// Matrix-representation of switches directly attached to the pins ("diodeless").
///
/// Generic parameters are in order: The type of column pins,
/// the number of columns and rows.
pub struct DirectPinMatrix<P, const CS: usize, const RS: usize>
where
    P: InputPin,
{
    pins: [[Option<P>; CS]; RS],
}

impl<P, const CS: usize, const RS: usize> DirectPinMatrix<P, CS, RS>
where
    P: InputPin,
{
    /// Creates a new DirectPinMatrix.
    ///
    /// Assumes pins are pull-up inputs,
    pub fn new<E>(pins: [[Option<P>; CS]; RS]) -> Result<Self, E>
    where
        P: InputPin<Error = E>,
    {
        let res = Self { pins };
        Ok(res)
    }

    /// Scans the pins and checks which keys are pressed (state is "low").
    pub fn get<E>(&mut self) -> Result<[[bool; CS]; RS], E>
    where
        P: InputPin<Error = E>,
    {
        let mut keys = [[false; CS]; RS];

        for (ri, row) in (&mut self.pins).iter_mut().enumerate() {
            for (ci, col_option) in row.iter().enumerate() {
                if let Some(col) = col_option {
                    if col.is_low()? {
                        keys[ri][ci] = true;
                    }
                }
            }
        }
        Ok(keys)
    }
}

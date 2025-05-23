import Std.Diagnostics.*;
import Std.Math.*;

/// # Summary
/// Converts a given integer `number` to an equivalent
/// double-precision floating-point number.
///
/// # Description
/// Converts a given integer to a double-precision floating point number.
/// Please note that the double-precision representation may have fewer
/// bits allocated to represent [significant digits](https://en.wikipedia.org/wiki/Significand)
/// so the conversion may be approximate for large numbers. For example,
/// the current simulator converts 4,611,686,018,427,387,919 = 2^64+15
/// to 4,611,686,018,427,387,904.0 = 2^64.
///
/// # Example
/// ```qsharp
/// Message($"{IntAsDouble(1)}"); // Prints 1.0 rather than 1
/// ```
function IntAsDouble(number : Int) : Double {
    body intrinsic;
}

/// # Summary
/// Converts a given integer `number` to an equivalent big integer.
function IntAsBigInt(number : Int) : BigInt {
    body intrinsic;
}

/// # Summary
/// Converts a `Result` type to a `Bool` type, where `One` is mapped to
/// `true` and `Zero` is mapped to `false`.
///
/// # Input
/// ## input
/// `Result` to be converted.
///
/// # Output
/// A `Bool` representing the `input`.
function ResultAsBool(input : Result) : Bool {
    input == One
}

/// # Summary
/// Converts a `Bool` type to a `Result` type, where `true` is mapped to
/// `One` and `false` is mapped to `Zero`.
///
/// # Input
/// ## input
/// `Bool` to be converted.
///
/// # Output
/// A `Result` representing the `input`.
function BoolAsResult(input : Bool) : Result {
    if input { One } else { Zero }
}

/// # Summary
/// Produces a non-negative integer from a string of bits in little-endian format.
/// `bits[0]` represents the least significant bit.
///
/// # Input
/// ## bits
/// Bits in binary representation of number.
function BoolArrayAsInt(bits : Bool[]) : Int {
    let nBits = Length(bits);
    Fact(nBits < 64, $"`Length(bits)` must be less than 64, but was {nBits}.");

    mutable number = 0;
    for i in 0..nBits - 1 {
        if (bits[i]) {
            set number |||= 1 <<< i;
        }
    }

    number
}

/// # Summary
/// Produces a binary representation of a non-negative integer, using the
/// little-endian representation for the returned array.
///
/// # Input
/// ## number
/// A non-negative integer to be converted to an array of Boolean values.
/// ## bits
/// The number of bits in the binary representation of `number`.
///
/// # Output
/// An array of Boolean values representing `number`.
///
/// # Remarks
/// The input `bits` must be non-negative.
/// The input `number` must be between 0 and 2^bits - 1.
function IntAsBoolArray(number : Int, bits : Int) : Bool[] {
    Fact(bits >= 0, "Requested number of bits must be non-negative.");
    Fact(number >= 0, "Number must be non-negative.");
    mutable runningValue = number;
    mutable result = [];
    for _ in 1..bits {
        set result += [(runningValue &&& 1) != 0];
        set runningValue >>>= 1;
    }
    Fact(runningValue == 0, "`number` is too large to fit into array of length `bits`.");

    result
}

/// # Summary
/// Converts an array of Boolean values into a non-negative BigInt, interpreting the
/// array as a binary representation in little-endian format.
///
/// # Input
/// ## boolArray
/// An array of Boolean values representing the binary digits of a BigInt.
///
/// # Output
/// A BigInt represented by `boolArray`.
///
/// # Remarks
/// The function interprets the array in little-endian format, where the first
/// element of the array represents the least significant bit.
/// The input `boolArray` should not be empty.
function BoolArrayAsBigInt(boolArray : Bool[]) : BigInt {
    mutable result = 0L;
    for i in 0..Length(boolArray) - 1 {
        if boolArray[i] {
            set result += 1L <<< i;
        }
    }

    result
}

/// # Summary
/// Produces a binary representation of a non-negative BigInt, using the
/// little-endian representation for the returned array.
///
/// # Input
/// ## number
/// A non-negative BigInt to be converted to an array of Boolean values.
/// ## bits
/// The number of bits in the binary representation of `number`.
///
/// # Output
/// An array of Boolean values representing `number`.
///
/// # Remarks
/// The input `bits` must be non-negative.
/// The input `number` must be between 0 and 2^bits - 1.
function BigIntAsBoolArray(number : BigInt, bits : Int) : Bool[] {
    Fact(bits >= 0, "Requested number of bits must be non-negative.");
    Fact(number >= 0L, "Number must be non-negative.");
    mutable runningValue = number;
    mutable result = [];
    for _ in 1..bits {
        set result += [(runningValue &&& 1L) != 0L];
        set runningValue >>>= 1;
    }
    Fact(runningValue == 0L, $"`number`={number} is too large to fit into {bits} bits.");

    result
}

/// # Summary
/// Converts a BigInt number into Int. Raises an error if the number is too large to fit.
///
/// # Input
/// ## number
/// A BigInt number to be converted.
///
/// # Output
/// Int representation of a number.
function BigIntAsInt(number : BigInt) : Int {
    let max = 9_223_372_036_854_775_807L;
    let min = -9_223_372_036_854_775_808L;
    Fact(number >= min and number <= max, $"BigIntAsInt: {number} is too big to fit into Int.");

    mutable result = 0;
    mutable powL = 1L;
    mutable pow = 1;
    for _ in 0..63 {
        if number &&& powL != 0L {
            result |||= pow;
        }
        powL <<<= 1;
        pow <<<= 1;
    }

    result
}

/// # Summary
/// Produces a non-negative integer from a string of Results in little-endian format.
///
/// # Input
/// ## results
/// Results in binary representation of number.
///
/// # Output
/// A non-negative integer
///
/// # Example
/// ```qsharp
/// // The following returns 1
/// let int1 = ResultArrayAsInt([One,Zero])
/// ```
function ResultArrayAsInt(results : Result[]) : Int {
    let nBits = Length(results);
    Fact(nBits < 64, $"`Length(bits)` must be less than 64, but was {nBits}.");

    mutable number = 0;
    for idxBit in 0..nBits - 1 {
        if (results[idxBit] == One) {
            set number |||= 1 <<< idxBit;
        }
    }

    number
}

/// # Summary
/// Converts a `Result[]` type to a `Bool[]` type, where `One`
/// is mapped to `true` and `Zero` is mapped to `false`.
///
/// # Input
/// ## input
/// `Result[]` to be converted.
///
/// # Output
/// A `Bool[]` representing the `input`.
function ResultArrayAsBoolArray(input : Result[]) : Bool[] {
    mutable output = [];
    for r in input {
        set output += [r == One];
    }

    output
}

/// # Summary
/// Converts a `Bool[]` type to a `Result[]` type, where `true`
/// is mapped to `One` and `false` is mapped to `Zero`.
///
/// # Input
/// ## input
/// `Bool[]` to be converted.
///
/// # Output
/// A `Result[]` representing the `input`.
function BoolArrayAsResultArray(input : Bool[]) : Result[] {
    mutable output = [];
    for b in input {
        set output += [if b { One } else { Zero }];
    }

    output
}

/// # Summary
/// Converts a complex number of type `Complex` to a complex
/// number of type `ComplexPolar`.
///
/// # Input
/// ## input
/// Complex number c = x + y𝑖.
///
/// # Output
/// Complex number c = r⋅e^(t𝑖).
function ComplexAsComplexPolar(input : Complex) : ComplexPolar {
    return ComplexPolar(AbsComplex(input), ArgComplex(input));
}

/// # Summary
/// Converts a complex number of type `ComplexPolar` to a complex
/// number of type `Complex`.
///
/// # Input
/// ## input
/// Complex number c = r⋅e^(t𝑖).
///
/// # Output
/// Complex number c = x + y𝑖.
function ComplexPolarAsComplex(input : ComplexPolar) : Complex {
    return Complex(
        input.Magnitude * Cos(input.Argument),
        input.Magnitude * Sin(input.Argument)
    );
}

/// # Summary
/// Converts a given double-precision floating-point number to a string representation with desired precision, rounding if required.
///
/// # Input
/// ## input
/// Double to be converted.
/// ## precision
/// Non-negative number of digits after the decimal point.
///
/// # Example
/// ```qsharp
/// Message($"{DoubleAsStringWithPrecision(0.354, 2)}"); // Prints 0.35
/// Message($"{DoubleAsStringWithPrecision(0.485, 1)}"); // Prints 0.5
/// Message($"{DoubleAsStringWithPrecision(5.6, 4)}"); // Prints 5.6000
/// Message($"{DoubleAsStringWithPrecision(2.268, 0)}"); // Prints 2
/// ```
function DoubleAsStringWithPrecision(input : Double, precision : Int) : String {
    body intrinsic;
}

export
    IntAsDouble,
    IntAsBigInt,
    ResultAsBool,
    BoolAsResult,
    BoolArrayAsInt,
    IntAsBoolArray,
    BoolArrayAsBigInt,
    BigIntAsBoolArray,
    BigIntAsInt,
    ResultArrayAsInt,
    ResultArrayAsBoolArray,
    BoolArrayAsResultArray,
    ComplexAsComplexPolar,
    ComplexPolarAsComplex,
    DoubleAsStringWithPrecision;

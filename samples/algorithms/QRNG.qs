/// # Sample
/// Quantum Random Number Generator
///
/// # Description
/// This program implements a quantum random number generator by setting qubits
/// in superposition and then using the measurement results as random bits.
namespace Sample {
    open Microsoft.Quantum.Convert;
    open Microsoft.Quantum.Intrinsic;
    open Microsoft.Quantum.Math;

    @EntryPoint()
    operation Main() : Int {
        let max = 100;
        Message($"Sampling a random number between 0 and {max}: ");

        // Generate random number in the 0..max range.
        return GenerateRandomNumberInRange(max);
    }

    /// # Summary
    /// Generates a ramdom number consisting of `bitSize` bits.
    operation GenerateRandomNumber(bitSize : Int) : Int {
        mutable bits = [];
        for idxBit in 1..bitSize {
            set bits += [GenerateRandomBit()];
        }
        ResultArrayAsInt(bits)
    }

    /// # Summary
    /// Generates a random number between 0 and `max`.
    operation GenerateRandomNumberInRange(max : Int) : Int {
        // Determine the number of bits needed to represent `max`
        let nBits = BitSizeI(max);
        while true {
            // Generate `nBits` random bits which will
            // represent the generated random number.
            let sample = GenerateRandomNumber(nBits);
            // Return random number if it is within the requested range.
            if sample <= max {
                return sample;
            }
            // Repeat the loop if the random number is outside the range.
        }
        0 // This code is never reached.
    }

    /// # Summary
    /// Generates a random bit.
    operation GenerateRandomBit() : Result {
        // Allocate a qubit.
        use q = Qubit();

        // Set the qubit into superposition of 0 and 1 using the Hadamard
        // operation `H`.
        H(q);

        // At this point the qubit `q` has 50% chance of being measured in the
        // |0〉 state and 50% chance of being measured in the |1〉 state.
        // Measure the qubit value using the `M` operation, and store the
        // measurement value in the `result` variable.
        let result = M(q);

        // Reset qubit to the |0〉 state.
        // Qubits must be in the |0〉 state by the time they are released.
        Reset(q);

        // Return the result of the measurement.
        return result;

        // Note that Qubit `q` is automatically released at the end of the block.
    }
}

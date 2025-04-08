// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use expect_test::expect;
use miette::Report;

use crate::tests::compile_qasm_to_qsharp_file;

/// These tests use manually constructed QASM with parens exprs
/// as there is a bug in the QASM parser with complex RHS exprs

#[test]
fn int_var_comparisons_can_be_translated() -> miette::Result<(), Vec<Report>> {
    let source = "
        int x = 5;
        int y = 3;
        bool f = (x > y);
        bool e = (x >= y);
        bool a = (x < y);
        bool c = (x <= y);
        bool b = (x == y);
        bool d = (x != y);
    ";

    let qsharp = compile_qasm_to_qsharp_file(source)?;
    expect![[r#"
        namespace qasm3_import {
            import QasmStd.Angle.*;
            import QasmStd.Convert.*;
            import QasmStd.Intrinsic.*;
            @EntryPoint()
            operation Test() : (Int, Int, Bool, Bool, Bool, Bool, Bool, Bool) {
                mutable x = 5;
                mutable y = 3;
                mutable f = (x > y);
                mutable e = (x >= y);
                mutable a = (x < y);
                mutable c = (x <= y);
                mutable b = (x == y);
                mutable d = (x != y);
                (x, y, f, e, a, c, b, d)
            }
        }"#]]
    .assert_eq(&qsharp);
    Ok(())
}

#[test]
fn uint_var_comparisons_can_be_translated() -> miette::Result<(), Vec<Report>> {
    let source = "
        uint x = 5;
        uint y = 3;
        bool f = (x > y);
        bool e = (x >= y);
        bool a = (x < y);
        bool c = (x <= y);
        bool b = (x == y);
        bool d = (x != y);
    ";

    let qsharp = compile_qasm_to_qsharp_file(source)?;
    expect![[r#"
        namespace qasm3_import {
            import QasmStd.Angle.*;
            import QasmStd.Convert.*;
            import QasmStd.Intrinsic.*;
            @EntryPoint()
            operation Test() : (Int, Int, Bool, Bool, Bool, Bool, Bool, Bool) {
                mutable x = 5;
                mutable y = 3;
                mutable f = (x > y);
                mutable e = (x >= y);
                mutable a = (x < y);
                mutable c = (x <= y);
                mutable b = (x == y);
                mutable d = (x != y);
                (x, y, f, e, a, c, b, d)
            }
        }"#]]
    .assert_eq(&qsharp);
    Ok(())
}

#[test]
fn bit_var_comparisons_can_be_translated() -> miette::Result<(), Vec<Report>> {
    let source = "
        bit x = 1;
        bit y = 0;
        bool f = (x > y);
        bool e = (x >= y);
        bool a = (x < y);
        bool c = (x <= y);
        bool b = (x == y);
        bool d = (x != y);
    ";

    let qsharp = compile_qasm_to_qsharp_file(source)?;
    expect![[r#"
        namespace qasm3_import {
            import QasmStd.Angle.*;
            import QasmStd.Convert.*;
            import QasmStd.Intrinsic.*;
            @EntryPoint()
            operation Test() : (Result, Result, Bool, Bool, Bool, Bool, Bool, Bool) {
                mutable x = One;
                mutable y = Zero;
                mutable f = (x > y);
                mutable e = (x >= y);
                mutable a = (x < y);
                mutable c = (x <= y);
                mutable b = (x == y);
                mutable d = (x != y);
                (x, y, f, e, a, c, b, d)
            }
        }"#]]
    .assert_eq(&qsharp);
    Ok(())
}

#[test]
fn bitarray_var_comparisons_can_be_translated() -> miette::Result<(), Vec<Report>> {
    let source = r#"
        bit[1] x = "1";
        bit[1] y = "0";
        bool f = (x > y);
        bool e = (x >= y);
        bool a = (x < y);
        bool c = (x <= y);
        bool b = (x == y);
        bool d = (x != y);
    "#;

    let qsharp = compile_qasm_to_qsharp_file(source)?;
    expect![[r#"
        namespace qasm3_import {
            import QasmStd.Angle.*;
            import QasmStd.Convert.*;
            import QasmStd.Intrinsic.*;
            @EntryPoint()
            operation Test() : (Result[], Result[], Bool, Bool, Bool, Bool, Bool, Bool) {
                mutable x = [One];
                mutable y = [Zero];
                mutable f = (__ResultArrayAsIntBE__(x) > __ResultArrayAsIntBE__(y));
                mutable e = (__ResultArrayAsIntBE__(x) >= __ResultArrayAsIntBE__(y));
                mutable a = (__ResultArrayAsIntBE__(x) < __ResultArrayAsIntBE__(y));
                mutable c = (__ResultArrayAsIntBE__(x) <= __ResultArrayAsIntBE__(y));
                mutable b = (__ResultArrayAsIntBE__(x) == __ResultArrayAsIntBE__(y));
                mutable d = (__ResultArrayAsIntBE__(x) != __ResultArrayAsIntBE__(y));
                (x, y, f, e, a, c, b, d)
            }
        }"#]]
    .assert_eq(&qsharp);
    Ok(())
}

#[test]
fn bitarray_var_comparison_to_int_can_be_translated() -> miette::Result<(), Vec<Report>> {
    let source = r#"
        bit[1] x = "1";
        input int y;
        bool a = (x > y);
        bool b = (x >= y);
        bool c = (x < y);
        bool d = (x <= y);
        bool e = (x == y);
        bool f = (x != y);
        bool g = (y > x);
        bool h = (y >= x);
        bool i = (y < x);
        bool j = (y <= x);
        bool k = (y == x);
        bool l = (y != x);
    "#;

    let qsharp = compile_qasm_to_qsharp_file(source)?;
    expect![[r#"
        namespace qasm3_import {
            import QasmStd.Angle.*;
            import QasmStd.Convert.*;
            import QasmStd.Intrinsic.*;
            operation Test(y : Int) : (Result[], Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool) {
                mutable x = [One];
                mutable a = (__ResultArrayAsIntBE__(x) > y);
                mutable b = (__ResultArrayAsIntBE__(x) >= y);
                mutable c = (__ResultArrayAsIntBE__(x) < y);
                mutable d = (__ResultArrayAsIntBE__(x) <= y);
                mutable e = (__ResultArrayAsIntBE__(x) == y);
                mutable f = (__ResultArrayAsIntBE__(x) != y);
                mutable g = (y > __ResultArrayAsIntBE__(x));
                mutable h = (y >= __ResultArrayAsIntBE__(x));
                mutable i = (y < __ResultArrayAsIntBE__(x));
                mutable j = (y <= __ResultArrayAsIntBE__(x));
                mutable k = (y == __ResultArrayAsIntBE__(x));
                mutable l = (y != __ResultArrayAsIntBE__(x));
                (x, a, b, c, d, e, f, g, h, i, j, k, l)
            }
        }"#]]
    .assert_eq(&qsharp);
    Ok(())
}

#[test]
fn bitarray_multiple_values_var_comparison_to_int_can_be_translated(
) -> miette::Result<(), Vec<Report>> {
    let source = r#"
        bit[3] x = "110";
        input int y;
        bool a = (x > y);
        bool b = (x >= y);
        bool c = (x < y);
        bool d = (x <= y);
        bool e = (x == y);
        bool f = (x != y);
        bool g = (y > x);
        bool h = (y >= x);
        bool i = (y < x);
        bool j = (y <= x);
        bool k = (y == x);
        bool l = (y != x);
    "#;

    let qsharp = compile_qasm_to_qsharp_file(source)?;
    expect![
        r#"
        namespace qasm3_import {
            operation Test(y : Int) : (Result[], Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool) {
                function __ResultArrayAsIntBE__(results : Result[]) : Int {
                    Microsoft.Quantum.Convert.ResultArrayAsInt(Microsoft.Quantum.Arrays.Reversed(results))
                }
                mutable x = [One, One, Zero];
                mutable a = (__ResultArrayAsIntBE__(x) > y);
                mutable b = (__ResultArrayAsIntBE__(x) >= y);
                mutable c = (__ResultArrayAsIntBE__(x) < y);
                mutable d = (__ResultArrayAsIntBE__(x) <= y);
                mutable e = (__ResultArrayAsIntBE__(x) == y);
                mutable f = (__ResultArrayAsIntBE__(x) != y);
                mutable g = (y > __ResultArrayAsIntBE__(x));
                mutable h = (y >= __ResultArrayAsIntBE__(x));
                mutable i = (y < __ResultArrayAsIntBE__(x));
                mutable j = (y <= __ResultArrayAsIntBE__(x));
                mutable k = (y == __ResultArrayAsIntBE__(x));
                mutable l = (y != __ResultArrayAsIntBE__(x));
                (x, a, b, c, d, e, f, g, h, i, j, k, l)
            }
        }"#
    ]
    .assert_eq(&qsharp);
    Ok(())
}

#[test]
fn float_var_comparisons_can_be_translated() -> miette::Result<(), Vec<Report>> {
    let source = "
        float x = 5;
        float y = 3;
        bool f = (x > y);
        bool e = (x >= y);
        bool a = (x < y);
        bool c = (x <= y);
        bool b = (x == y);
        bool d = (x != y);
    ";

    let qsharp = compile_qasm_to_qsharp_file(source)?;
    expect![[r#"
        namespace qasm3_import {
            import QasmStd.Angle.*;
            import QasmStd.Convert.*;
            import QasmStd.Intrinsic.*;
            @EntryPoint()
            operation Test() : (Double, Double, Bool, Bool, Bool, Bool, Bool, Bool) {
                mutable x = 5.;
                mutable y = 3.;
                mutable f = (x > y);
                mutable e = (x >= y);
                mutable a = (x < y);
                mutable c = (x <= y);
                mutable b = (x == y);
                mutable d = (x != y);
                (x, y, f, e, a, c, b, d)
            }
        }"#]]
    .assert_eq(&qsharp);
    Ok(())
}

#[test]
fn bool_var_comparisons_can_be_translated() -> miette::Result<(), Vec<Report>> {
    let source = "
        bool x = true;
        bool y = false;
        bool a = (x && y);
        bool b = (x || y);
        bool c = (!x && !y);
        bool d = (!x || !y);
        bool e = (!x && y);
        bool f = (!x || y);
        bool g = (x && !y);
        bool h = (x || !y);
    ";

    let qsharp = compile_qasm_to_qsharp_file(source)?;
    expect![[r#"
        namespace qasm3_import {
            import QasmStd.Angle.*;
            import QasmStd.Convert.*;
            import QasmStd.Intrinsic.*;
            @EntryPoint()
            operation Test() : (Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool) {
                mutable x = true;
                mutable y = false;
                mutable a = (x and y);
                mutable b = (x or y);
                mutable c = (not x and not y);
                mutable d = (not x or not y);
                mutable e = (not x and y);
                mutable f = (not x or y);
                mutable g = (x and not y);
                mutable h = (x or not y);
                (x, y, a, b, c, d, e, f, g, h)
            }
        }"#]]
    .assert_eq(&qsharp);
    Ok(())
}

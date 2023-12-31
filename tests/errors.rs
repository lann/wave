use std::sync::{Mutex, OnceLock};

use wasmtime::{
    component::{Component, Instance, Linker, Type, Val},
    Config, Engine, Store,
};

#[test]
fn test_float_errors() {
    for (func, input) in [
        // Reject "-nan".
        ("floats", "(-nan, 0.0)"),
        ("floats", "(0.0, -nan)"),
        // Reject "infinity", "-infinity", and uppercase variations.
        ("floats", "(0.0, infinity)"),
        ("floats", "(0.0, -infinity)"),
        ("floats", "(infinity, 0.0)"),
        ("floats", "(-infinity, 0.0)"),
        ("floats", "(0.0, INFINITY)"),
        ("floats", "(0.0, -INFINITY)"),
        ("floats", "(INFINITY, 0.0)"),
        ("floats", "(-INFINITY, 0.0)"),
        ("floats", "(0.0, Infinity)"),
        ("floats", "(0.0, -Infinity)"),
        ("floats", "(Infinity, 0.0)"),
        ("floats", "(-Infinity, 0.0)"),
        // Reject uppercase variations of "inf" and "nan".
        ("floats", "(0.0, Inf)"),
        ("floats", "(0.0, -Inf)"),
        ("floats", "(Inf, 0.0)"),
        ("floats", "(-Inf, 0.0)"),
        ("floats", "(0.0, INF)"),
        ("floats", "(0.0, -INF)"),
        ("floats", "(INF, 0.0)"),
        ("floats", "(-INF, 0.0)"),
        ("floats", "(0.0, NaN)"),
        ("floats", "(NaN, 0.0)"),
        ("floats", "(0.0, NAN)"),
        ("floats", "(NAN, 0.0)"),
    ] {
        assert_reject(func, input);
    }
}

#[test]
fn test_string_errors() {
    for (func, input) in [
        // Reject surrogates.
        ("list-strings", "[\"\\u{d800}\"]"),
        ("list-strings", "[\"\\u{dbff}\"]"),
        ("list-strings", "[\"\\u{dc00}\"]"),
        ("list-strings", "[\"\\u{dcff}\"]"),
        ("list-strings", "[\"\\u{d800}\\u{dc00}\"]"),
        // Reject invalid values.
        ("list-strings", "[\"\\u{110000}\"]"),
        ("list-strings", "[\"\\u{ffffffff}\"]"),
        ("list-strings", "[\"\\u{80000000}\"]"),
        // Reject invalid syntax.
        ("list-strings", "[\"\\u{-1}\"]"),
        ("list-strings", "[\"\\u{+1}\"]"),
    ] {
        assert_reject(func, input);
    }
}

#[test]
fn test_option_errors() {
    for (func, input) in [
        ("options", "(some, some(some(0)))"),
        ("options", "(some(), some(some(0)))"),
        ("options", "(some(0), some(some))"),
        ("options", "(some(0), some(some()))"),
        ("options", "(some(some(0)), some(some(0)))"),
        ("options", "(some(0), some(some(some(0))))"),
        ("options", "(none(), some(some(0)))"),
        ("options", "(some(0), some(none()))"),
        ("options", "(none(0), some(some(0)))"),
        ("options", "(some(0), some(none(0)))"),
        ("options", "(some(0), none(some(0)))"),
    ] {
        assert_reject(func, input);
    }
}

#[test]
fn test_result_errors() {
    for (func, input) in [
        ("result-ok-only", "ok"),
        ("result-ok-only", "ok()"),
        ("result-ok-only", "o(0)"),
        ("result-ok-only", "err(0)"),
        ("result-err-only", "err"),
        ("result-err-only", "err()"),
        ("result-err-only", "e(0)"),
        ("result-err-only", "ok(0)"),
        ("result-no-payloads", "ok()"),
        ("result-no-payloads", "o(0)"),
        ("result-no-payloads", "ok(0)"),
        ("result-no-payloads", "err()"),
        ("result-no-payloads", "e(0)"),
        ("result-no-payloads", "err(0)"),
        ("result-both-payloads", "ok"),
        ("result-both-payloads", "ok()"),
        ("result-both-payloads", "o(0)"),
        ("result-both-payloads", "err"),
        ("result-both-payloads", "err()"),
        ("result-both-payloads", "e(0)"),
    ] {
        assert_reject(func, input);
    }
}

#[test]
fn test_record_errors() {
    // Missing `required`.
    assert_reject("record", "{}");
    assert_reject("record", "{ optional: none }");
    assert_reject("record", "{ optional: some(0) }");

    // Duplicate `required`.
    assert_reject("record", "{ required: 0, required: 0 }");
    assert_reject("record", "{ required: 0, required: 0, optional: none }");
    assert_reject("record", "{ required: 0, required: 0, optional: some(0) }");

    // Duplicate `optional`.
    assert_reject("record", "{ required: 0, optional: none, optional: none }");
    assert_reject(
        "record",
        "{ required: 0, optional: none, optional: some(0) }",
    );
    assert_reject(
        "record",
        "{ required: 0, optional: some(0), optional: none }",
    );
    assert_reject(
        "record",
        "{ required: 0, optional: some(0), optional: some(0) }",
    );

    // Bad commas.
    assert_reject("record", "{ required: 0,, }");
    assert_reject("record", "{ , required: 0, }");
    assert_reject("record", "{ ,, required: 0 }");
}

#[test]
fn test_flags_errors() {
    // Duplicate flags.
    assert_reject("flags", "{ read, read }");
    assert_reject("flags", "{ write, write }");
    assert_reject("flags", "{ read, write, read }");

    // Unrecognized flag.
    assert_reject("flags", "{ read, write, execute }");
}

#[test]
fn test_list_errors() {
    for (func, input) in [
        ("list-strings", "[\"\\u{0}\",,]"),
        ("list-strings", "[,\"\\u{0}\"]"),
        ("list-strings", "[,]"),
        ("list-strings", "[)"),
        ("list-strings", "(]"),
        ("list-strings", "[}"),
        ("list-strings", "{]"),
    ] {
        assert_reject(func, input);
    }
}

fn assert_reject(type_name: &str, input: &str) {
    let ty = get_type(type_name);
    let result: Result<Val, wasm_wave::parser::ParserError> = wasm_wave::from_str(&ty, input);
    match result {
        Ok(got) => panic!("failed to reject {input:?} as type {type_name}; got '{got:?}'"),
        Err(err) => {
            dbg!(err);
        }
    }
}

fn get_type(name: &str) -> Type {
    static INSTANCE_AND_STORE: OnceLock<(Instance, Mutex<Store<()>>)> = OnceLock::new();
    let (instance, store) = INSTANCE_AND_STORE.get_or_init(|| {
        let engine = Engine::new(Config::new().wasm_component_model(true)).expect("engine");
        let component = Component::from_file(&engine, "tests/types.wasm").expect("component");
        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        let instance = linker
            .instantiate(&mut store, &component)
            .expect("instance");
        (instance, Mutex::new(store))
    });
    let mut store = store.lock().unwrap();
    let func = instance
        .exports(&mut *store)
        .root()
        .func(name)
        .unwrap_or_else(|| panic!("export func named {name:?}"));
    func.results(&*store)[0].clone()
}

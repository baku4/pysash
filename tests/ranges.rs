//! `Range`는 사용자가 준 **원본 바이트열** 기준이어야 한다. 이게 어긋나면 잘라낸
//! 코드를 인터프리터에 먹일 수 없다.

use pysash::Range;
use pysash::source::ParseErrorKind;
use pysash::source::PythonSource;

fn ranges(source: &str) -> Vec<(u32, u32)> {
    PythonSource::parse(source)
        .unwrap_or_else(|e| panic!("{source:?}: {e}"))
        .statements()
        .iter()
        .map(|s| (s.range.start, s.range.end))
        .collect()
}

/// 선행 주석은 statement에 포함되지 않고, 데코레이터는 포함된다.
#[test]
fn range_excludes_leading_comment_and_includes_decorator() {
    assert_eq!(ranges("# lead\n@deco\ndef f():\n    pass\n"), [(7, 30)]);
}

#[test]
fn range_splits_statements_on_the_same_line() {
    assert_eq!(ranges("a = 1; b = 2\n"), [(0, 5), (7, 12)]);
}

/// offset은 문자 수가 아니라 바이트 수다.
#[test]
fn range_is_measured_in_bytes() {
    assert_eq!(ranges("s = '한글테스트'\nt = 2\n"), [(0, 21), (22, 27)]);
}

/// BOM을 떼어낸 만큼 offset이 밀린다 — 원본 기준을 유지해야 하기 때문이다.
#[test]
fn range_accounts_for_a_byte_order_mark() {
    let source = "\u{FEFF}x = 1\n";
    let parsed = PythonSource::parse(source).unwrap();

    assert_eq!(parsed.statements()[0].range, Range::new(3, 8));
    assert_eq!(parsed.slice(parsed.statements()[0].range), b"x = 1");
}

/// range로 잘라낸 바이트를 다시 파싱하면 같은 statement가 나온다.
#[test]
fn slicing_a_statement_round_trips_to_the_same_identity() {
    let source = "import os\nx = 1000  # comment\n\ndef f(a):\n    return a + 1\n\nif x: pass\n";
    let parsed = PythonSource::parse(source).unwrap();
    assert_eq!(parsed.statements().len(), 4);

    for statement in parsed.statements() {
        let text = str::from_utf8(parsed.slice(statement.range)).unwrap();
        let reparsed = PythonSource::parse(text).unwrap();
        assert_eq!(reparsed.statements().len(), 1, "{text:?}");
        assert_eq!(
            reparsed.statements()[0].canonical,
            statement.canonical,
            "{text:?}"
        );
    }
}

#[test]
fn raw_bytes_are_preserved_verbatim() {
    let source = "x = 1  # comment\n";
    let parsed = PythonSource::parse(source).unwrap();

    assert_eq!(parsed.raw(), source.as_bytes());
}

#[test]
fn syntax_errors_are_the_only_failure_of_parsing() {
    let error = PythonSource::parse("def f(:\n").unwrap_err();
    assert!(matches!(error.kind, ParseErrorKind::Syntax { .. }));

    // 파싱된다는 것이 실행이 성공한다는 뜻은 아니다 — 이것들은 전부 Ok다.
    for source in [
        "undefined_name\n",
        "1/0\n",
        "import nonexistent_module_xyz\n",
    ] {
        assert!(PythonSource::parse(source).is_ok(), "{source:?}");
    }
}

#[test]
fn non_utf8_input_is_rejected() {
    let error = PythonSource::parse_bytes(b"x = '\xff'\n").unwrap_err();
    assert!(matches!(error.kind, ParseErrorKind::NotUtf8 { offset: 5 }));
}

/// UTF-8이 아닌 인코딩 선언은 거부한다. `Range`가 어느 바이트열 기준인지
/// 흐려지느니 명시적으로 실패하는 편이 낫다.
#[test]
fn non_utf8_coding_cookie_is_rejected() {
    let error = PythonSource::parse("# -*- coding: latin-1 -*-\nx = 1\n").unwrap_err();
    match error.kind {
        ParseErrorKind::UnsupportedEncoding { declared } => assert_eq!(&*declared, "latin-1"),
        other => panic!("expected UnsupportedEncoding, got {other:?}"),
    }

    assert!(PythonSource::parse("# -*- coding: utf-8 -*-\nx = 1\n").is_ok());
    assert!(PythonSource::parse("#!/usr/bin/env python\n# coding=utf8\nx = 1\n").is_ok());
}

#[test]
fn an_empty_source_has_no_statements() {
    assert!(PythonSource::parse("").unwrap().statements().is_empty());
    assert!(
        PythonSource::parse("\n\n# only a comment\n")
            .unwrap()
            .statements()
            .is_empty()
    );
}

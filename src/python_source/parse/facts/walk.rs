use ruff_python_ast::visitor::{Visitor, walk_expr, walk_pattern, walk_stmt};
// module-rule: allow import-alias
use ruff_python_ast as ast;
use crate::statement_facts::CalleeSummary;
use super::scan::MentionScan;

/// 지금 실행되는 수준의 walk.
///
/// def/lambda 본문은 지금 실행되지 않으므로 내려가지 않고 요약으로 접는다.
/// class 본문은 지금 실행되므로 내려간다. comprehension 변수는 밖으로 새지 않게
/// 가린다.
#[derive(Default)]
pub struct ExecWalker {
    pub sink: Sink,
    /// 현재 열려 있는 comprehension들의 지역 이름.
    pub shield: Vec<String>,
}

/// pass 2 — 지금 실행되는 수준의 walk가 모은 원시 사실들.
///
/// module 수준에서 걸었으면 그대로 facts의 재료가 되고, 함수 본문에서 걸었으면
/// locals를 알아낸 뒤 [`CalleeSummary`]로 접힌다.
#[derive(Default)]
pub struct Sink {
    pub binds: Vec<String>,
    pub reads: Vec<String>,
    pub calls: Vec<String>,
    pub mutates: Vec<String>,
    pub deletes: Vec<String>,
    pub alias_edges: Vec<(String, String)>,
    pub global_decls: Vec<String>,
    /// 이름에 묶인 callable(def / class / `f = lambda`)의 요약.
    pub nested: Vec<CalleeSummary>,
    /// 이름에 묶이지 않은 callable(인자로 넘긴 lambda 등)의 요약.
    pub loose: Vec<CalleeSummary>,
    pub opaque: bool,
}

impl ExecWalker {
    pub fn run(stmt: &ast::Stmt) -> Sink {
        let mut walker = ExecWalker::default();
        walker.visit_stmt(stmt);
        walker.sink
    }

    fn shielded(&self, name: &str) -> bool {
        self.shield.iter().any(|shielded| shielded == name)
    }

    fn bind(&mut self, name: &str) {
        if !self.shielded(name) {
            push_unique(&mut self.sink.binds, name);
        }
    }

    fn read(&mut self, name: &str) {
        if !self.shielded(name) {
            push_unique(&mut self.sink.reads, name);
        }
    }

    fn mutate(&mut self, expr: &ast::Expr) {
        if let Some(root) = root_name(expr) {
            push_unique(&mut self.sink.mutates, root);
        }
    }

    fn visit_parameter_declarations(&mut self, parameters: &ast::Parameters) {
        // default와 annotation은 def를 실행하는 지금 이 자리에서 평가된다.
        for parameter in parameters.iter() {
            if let Some(default) = parameter.default() {
                self.visit_expr(default);
            }
            if let Some(annotation) = parameter.annotation() {
                self.visit_expr(annotation);
            }
        }
    }

    fn visit_decorators(&mut self, decorators: &[ast::Decorator]) {
        for decorator in decorators {
            // `@register`는 def 시점에 register를 호출한다. Call 형태(@app.route(...))는
            // Call 방문이 처리하므로 bare 이름만 여기서 호출로 등록한다.
            if let ast::Expr::Name(name) = &decorator.expression {
                push_unique(&mut self.sink.calls, name.id.as_str());
            }
            self.visit_expr(&decorator.expression);
        }
    }
}

impl<'a> Visitor<'a> for ExecWalker {
    fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
        match stmt {
            ast::Stmt::FunctionDef(def) => {
                self.bind(def.name.as_str());
                self.visit_decorators(&def.decorator_list);
                self.visit_parameter_declarations(&def.parameters);
                if let Some(returns) = &def.returns {
                    self.visit_expr(returns);
                }
                let (summary, frees) = analyze_stmts(Some(&def.parameters), &def.body);
                for name in &frees {
                    self.read(name);
                }
                self.sink.nested.push(summary);
            }
            ast::Stmt::ClassDef(class) => {
                self.bind(class.name.as_str());
                self.visit_decorators(&class.decorator_list);
                if let Some(arguments) = &class.arguments {
                    for base in &arguments.args {
                        // 상속은 구문상 대응물이 없는 별칭 간선이다 — 상속된 mutable
                        // 속성은 부모와 공유 객체다.
                        if let ast::Expr::Name(base_name) = base {
                            self.sink
                                .alias_edges
                                .push((class.name.to_string(), base_name.id.to_string()));
                        }
                        self.visit_expr(base);
                    }
                    for keyword in &arguments.keywords {
                        self.visit_expr(&keyword.value);
                    }
                }
                // class 본문은 지금 실행된다. 다만 본문의 바인딩은 module 이름이
                // 아니라 class 속성이다 — 바인딩만 버리고 나머지는 흡수한다.
                let mut inner = ExecWalker::default();
                for body_stmt in &class.body {
                    inner.visit_stmt(body_stmt);
                }
                let inner = inner.sink;
                for name in &inner.reads {
                    self.read(name);
                }
                for name in &inner.calls {
                    push_unique(&mut self.sink.calls, name);
                }
                for name in &inner.mutates {
                    push_unique(&mut self.sink.mutates, name);
                }
                // `class C: global g; g = 1`의 g는 지금 module에 바인딩된다.
                for name in &inner.binds {
                    if inner.global_decls.contains(name) {
                        self.bind(name);
                    }
                }
                self.sink.opaque |= inner.opaque;
                // method들의 요약이 곧 이 class의 요약이다 — C()를 부르면 그중
                // 무엇이든 실행될 수 있다.
                let mut class_summary = CalleeSummary::default();
                for nested in inner.nested.iter().chain(&inner.loose) {
                    class_summary.absorb(nested);
                }
                self.sink.nested.push(class_summary);
            }
            ast::Stmt::Assign(assign) => {
                self.visit_expr(&assign.value);
                if let [ast::Expr::Name(target)] = &assign.targets[..] {
                    // `b = a`는 별칭 간선이다. b를 통한 변경이 a에 보인다.
                    if let ast::Expr::Name(value) = &*assign.value {
                        self.sink
                            .alias_edges
                            .push((target.id.to_string(), value.id.to_string()));
                    }
                    // `f = lambda ...`는 이름에 묶인 callable이다.
                    if matches!(&*assign.value, ast::Expr::Lambda(_))
                        && let Some(summary) = self.sink.loose.pop()
                    {
                        self.sink.nested.push(summary);
                    }
                }
                for target in &assign.targets {
                    self.visit_expr(target);
                }
            }
            ast::Stmt::AugAssign(assign) => {
                self.visit_expr(&assign.value);
                match &*assign.target {
                    ast::Expr::Name(name) => {
                        let id = name.id.as_str();
                        self.read(id);
                        self.bind(id);
                        // `x += [1]`은 list라면 in-place다.
                        push_unique(&mut self.sink.mutates, id);
                    }
                    target => self.visit_expr(target),
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.visit_expr(&for_stmt.iter);
                // generator를 순회하면 소모된다.
                self.mutate(&for_stmt.iter);
                self.visit_expr(&for_stmt.target);
                for body_stmt in &for_stmt.body {
                    self.visit_stmt(body_stmt);
                }
                for body_stmt in &for_stmt.orelse {
                    self.visit_stmt(body_stmt);
                }
            }
            ast::Stmt::Import(import) => {
                for alias in &import.names {
                    match &alias.asname {
                        Some(asname) => self.bind(asname.as_str()),
                        // `import a.b.c`는 a를 바인딩한다.
                        None => {
                            let root = alias.name.split('.').next().unwrap_or_default();
                            self.bind(root);
                        }
                    }
                }
            }
            ast::Stmt::ImportFrom(import) => {
                for alias in &import.names {
                    if alias.name.as_str() == "*" {
                        self.sink.opaque = true;
                        continue;
                    }
                    match &alias.asname {
                        Some(asname) => self.bind(asname.as_str()),
                        None => self.bind(alias.name.as_str()),
                    }
                }
            }
            ast::Stmt::Global(global) => {
                for name in &global.names {
                    push_unique(&mut self.sink.global_decls, name.as_str());
                }
            }
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a ast::Expr) {
        match expr {
            ast::Expr::Name(name) => {
                let id = name.id.as_str();
                match name.ctx {
                    ast::ExprContext::Load => self.read(id),
                    ast::ExprContext::Store => self.bind(id),
                    ast::ExprContext::Del => {
                        if !self.shielded(id) {
                            push_unique(&mut self.sink.deletes, id);
                        }
                    }
                    ast::ExprContext::Invalid => {}
                }
            }
            ast::Expr::Attribute(attribute) => {
                if matches!(
                    attribute.ctx,
                    ast::ExprContext::Store | ast::ExprContext::Del
                ) {
                    self.mutate(&attribute.value);
                }
                walk_expr(self, expr);
            }
            ast::Expr::Subscript(subscript) => {
                if matches!(
                    subscript.ctx,
                    ast::ExprContext::Store | ast::ExprContext::Del
                ) {
                    self.mutate(&subscript.value);
                }
                walk_expr(self, expr);
            }
            ast::Expr::Call(call) => {
                let pure = match &*call.func {
                    ast::Expr::Name(func) => {
                        let id = func.id.as_str();
                        push_unique(&mut self.sink.calls, id);
                        pure_call(id)
                    }
                    ast::Expr::Attribute(func) => {
                        // method 호출은 receiver를 바꿀 수 있다.
                        self.mutate(&func.value);
                        false
                    }
                    _ => false,
                };
                self.visit_expr(&call.func);
                for arg in &call.arguments.args {
                    self.visit_expr(arg);
                    if !pure {
                        // 인자로 넘기는 것만으로 mutation 후보가 된다.
                        let inner = match arg {
                            ast::Expr::Starred(starred) => &*starred.value,
                            other => other,
                        };
                        self.mutate(inner);
                    }
                }
                for keyword in &call.arguments.keywords {
                    self.visit_expr(&keyword.value);
                    if !pure {
                        self.mutate(&keyword.value);
                    }
                }
            }
            ast::Expr::Lambda(lambda) => {
                if let Some(parameters) = &lambda.parameters {
                    self.visit_parameter_declarations(parameters);
                }
                let (summary, frees) =
                    analyze_expr(lambda.parameters.as_deref(), &lambda.body);
                for name in &frees {
                    self.read(name);
                }
                self.sink.loose.push(summary);
            }
            ast::Expr::ListComp(_)
            | ast::Expr::SetComp(_)
            | ast::Expr::DictComp(_)
            | ast::Expr::Generator(_) => {
                let mark = self.shield.len();
                let generators = match expr {
                    ast::Expr::ListComp(comp) => &comp.generators,
                    ast::Expr::SetComp(comp) => &comp.generators,
                    ast::Expr::DictComp(comp) => &comp.generators,
                    ast::Expr::Generator(comp) => &comp.generators,
                    _ => unreachable!(),
                };
                for generator in generators {
                    collect_names(&generator.target, &mut self.shield);
                }
                walk_expr(self, expr);
                self.shield.truncate(mark);
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(inner) = handler;
        if let Some(name) = &inner.name {
            // `except E as e`의 e는 블록이 끝나면 지워진다 — 바인딩이자 삭제다.
            self.bind(name.as_str());
            push_unique(&mut self.sink.deletes, name.as_str());
        }
        if let Some(type_) = &inner.type_ {
            self.visit_expr(type_);
        }
        for stmt in &inner.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        for name in pattern_binds(pattern) {
            self.bind(name);
        }
        walk_pattern(self, pattern);
    }
}

/// def 본문을 요약한다. 본문은 지금 실행되지 않는다 — 나중에 호출될 때 무슨 일이
/// 일어날 수 있는지의 상계와, 본문이 읽는 free name들을 돌려준다.
fn analyze_stmts(
    parameters: Option<&ast::Parameters>,
    body: &[ast::Stmt],
) -> (CalleeSummary, Vec<String>) {
    let mut walker = ExecWalker::default();
    let mut scan = MentionScan::default();
    for stmt in body {
        walker.visit_stmt(stmt);
        scan.visit_stmt(stmt);
    }
    finish_callable(walker.sink, scan.opaque, parameters)
}

/// lambda 본문을 요약한다.
fn analyze_expr(
    parameters: Option<&ast::Parameters>,
    body: &ast::Expr,
) -> (CalleeSummary, Vec<String>) {
    let mut walker = ExecWalker::default();
    let mut scan = MentionScan::default();
    walker.visit_expr(body);
    scan.visit_expr(body);
    finish_callable(walker.sink, scan.opaque, parameters)
}

fn finish_callable(
    mut sink: Sink,
    body_reflective: bool,
    parameters: Option<&ast::Parameters>,
) -> (CalleeSummary, Vec<String>) {
    // 본문의 반사 구문은 이 함수를 호출하는 것을 opaque로 만든다.
    sink.opaque |= body_reflective;

    let positional: Vec<&str> = parameters
        .map(|parameters| {
            parameters
                .posonlyargs
                .iter()
                .chain(&parameters.args)
                .map(|parameter| parameter.parameter.name.as_str())
                .collect()
        })
        .unwrap_or_default();
    let mut all_params: Vec<&str> = positional.clone();
    if let Some(parameters) = parameters {
        for parameter in &parameters.kwonlyargs {
            all_params.push(parameter.parameter.name.as_str());
        }
        if let Some(vararg) = &parameters.vararg {
            all_params.push(vararg.name.as_str());
        }
        if let Some(kwarg) = &parameters.kwarg {
            all_params.push(kwarg.name.as_str());
        }
    }

    // 본문 어딘가에서 대입되는 이름은 (global 선언이 없는 한) 본문 전체에서 local이다.
    let locals: Vec<&str> = all_params
        .iter()
        .copied()
        .chain(
            sink.binds
                .iter()
                .map(String::as_str)
                .filter(|name| !sink.global_decls.iter().any(|global| global == name)),
        )
        .collect();
    let is_local = |name: &str| locals.contains(&name);

    let mut summary = CalleeSummary {
        callees: sink.calls.iter().map(|name| name.as_str().into()).collect(),
        opaque: sink.opaque,
        ..CalleeSummary::default()
    };
    for name in sink.binds.iter().chain(&sink.deletes) {
        if sink.global_decls.iter().any(|global| global == name) {
            push_unique_boxed(&mut summary.global_writes, name);
        }
    }
    for name in &sink.mutates {
        if let Some(position) = positional.iter().position(|parameter| parameter == name) {
            if !summary.mutates_params.contains(&position) {
                summary.mutates_params.push(position);
            }
        } else if !is_local(name) {
            push_unique_boxed(&mut summary.mutates_frees, name);
        }
    }
    // 본문 안에서 정의된 callable이 하는 일도 이 함수를 호출하면 일어날 수 있다.
    for nested in sink.nested.iter().chain(&sink.loose) {
        for name in &nested.global_writes {
            push_unique_boxed(&mut summary.global_writes, name);
        }
        for name in &nested.mutates_frees {
            if !is_local(name) {
                push_unique_boxed(&mut summary.mutates_frees, name);
            }
        }
        for name in &nested.callees {
            if !is_local(name) {
                push_unique_boxed(&mut summary.callees, name);
            }
        }
        summary.opaque |= nested.opaque;
    }

    let frees = sink
        .reads
        .iter()
        .filter(|name| !is_local(name))
        .cloned()
        .collect();
    (summary, frees)
}

/// 인자를 in-place로 바꾸지 않는다고 믿는 호출인가. 이 밖의 모든 호출은 receiver와
/// 인자 전부를 mutation 후보로 잡는다 — 틀리면 Run이 늘어날 뿐인 방향이다.
fn pure_call(name: &str) -> bool {
    matches!(
        name,
        "abs" | "all"
            | "any"
            | "bool"
            | "callable"
            | "chr"
            | "dict"
            | "divmod"
            | "enumerate"
            | "float"
            | "format"
            | "frozenset"
            | "hash"
            | "hex"
            | "id"
            | "int"
            | "isinstance"
            | "issubclass"
            | "iter"
            | "len"
            | "list"
            | "map"
            | "max"
            | "min"
            | "oct"
            | "ord"
            | "print"
            | "range"
            | "repr"
            | "reversed"
            | "round"
            | "set"
            | "sorted"
            | "str"
            | "sum"
            | "tuple"
            | "type"
            | "zip"
    )
}

/// `x.a.b[k]` 같은 접근 사슬의 뿌리 이름.
fn root_name(expr: &ast::Expr) -> Option<&str> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.as_str()),
        ast::Expr::Attribute(attribute) => root_name(&attribute.value),
        ast::Expr::Subscript(subscript) => root_name(&subscript.value),
        ast::Expr::Starred(starred) => root_name(&starred.value),
        _ => None,
    }
}

/// 대입 target 안의 모든 이름 (`(a, b), c` 같은 unpack 포함).
fn collect_names(expr: &ast::Expr, out: &mut Vec<String>) {
    match expr {
        ast::Expr::Name(name) => out.push(name.id.to_string()),
        ast::Expr::Tuple(tuple) => {
            for element in &tuple.elts {
                collect_names(element, out);
            }
        }
        ast::Expr::List(list) => {
            for element in &list.elts {
                collect_names(element, out);
            }
        }
        ast::Expr::Starred(starred) => collect_names(&starred.value, out),
        _ => {}
    }
}

/// match pattern이 바인딩하는 이름들.
fn pattern_binds(pattern: &ast::Pattern) -> Vec<&str> {
    let mut names = Vec::new();
    fn walk<'a>(pattern: &'a ast::Pattern, out: &mut Vec<&'a str>) {
        match pattern {
            ast::Pattern::MatchAs(as_pattern) => {
                if let Some(name) = &as_pattern.name {
                    out.push(name.as_str());
                }
                if let Some(inner) = &as_pattern.pattern {
                    walk(inner, out);
                }
            }
            ast::Pattern::MatchStar(star) => {
                if let Some(name) = &star.name {
                    out.push(name.as_str());
                }
            }
            ast::Pattern::MatchMapping(mapping) => {
                if let Some(rest) = &mapping.rest {
                    out.push(rest.as_str());
                }
                for inner in &mapping.patterns {
                    walk(inner, out);
                }
            }
            ast::Pattern::MatchSequence(sequence) => {
                for inner in &sequence.patterns {
                    walk(inner, out);
                }
            }
            ast::Pattern::MatchOr(or_pattern) => {
                for inner in &or_pattern.patterns {
                    walk(inner, out);
                }
            }
            ast::Pattern::MatchClass(class) => {
                for inner in &class.arguments.patterns {
                    walk(inner, out);
                }
                for keyword in &class.arguments.keywords {
                    walk(&keyword.pattern, out);
                }
            }
            ast::Pattern::MatchValue(_) | ast::Pattern::MatchSingleton(_) => {}
        }
    }
    walk(pattern, &mut names);
    names
}

fn push_unique(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|existing| existing == name) {
        names.push(name.to_string());
    }
}

fn push_unique_boxed(names: &mut Vec<Box<str>>, name: &str) {
    if !names.iter().any(|existing| &**existing == name) {
        names.push(name.into());
    }
}

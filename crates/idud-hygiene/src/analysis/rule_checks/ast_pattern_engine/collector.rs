use syn::{
    visit::{self, Visit},
    Attribute, Expr, ExprCall, ExprMacro, ExprMethodCall, ExprPath, ItemFn, ItemForeignMod,
    ItemMod,
};

#[derive(Debug, Default)]
pub(crate) struct SemanticTokenCollector {
    pub tokens: Vec<SemanticToken>,
}

#[derive(Debug, Clone)]
pub(crate) enum SemanticToken {
    Path(Vec<String>),
    Macro(String),
    FunctionCall(String),
    Call {
        path: Vec<String>,
        args: Vec<String>,
    },
    MethodCall {
        receiver: Option<String>,
        method: String,
    },
    Identifier(String),
    Module(String),
    Keyword(String),
    Attribute {
        path: Vec<String>,
        args: Vec<String>,
    },
}

impl<'ast> Visit<'ast> for SemanticTokenCollector {
    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.tokens.push(SemanticToken::Path(path_segments(node)));
        visit::visit_path(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast ExprPath) {
        visit::visit_expr_path(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        self.tokens
            .push(SemanticToken::Path(path_segments(&node.mac.path)));
        if let Some(name) = macro_name(&node.mac.path) {
            self.tokens.push(SemanticToken::Macro(name));
        }
        visit::visit_expr_macro(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(func_path) = &*node.func {
            let path = path_segments(&func_path.path);
            let args = node
                .args
                .iter()
                .filter_map(expr_to_string)
                .collect::<Vec<_>>();
            self.tokens.push(SemanticToken::Call {
                path: path.clone(),
                args: args.clone(),
            });
            if let Some(ident) = func_path.path.get_ident() {
                self.tokens
                    .push(SemanticToken::FunctionCall(ident.to_string()));
            }
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        self.tokens.push(SemanticToken::MethodCall {
            receiver: receiver_name(&*node.receiver),
            method: node.method.to_string(),
        });
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.tokens
            .push(SemanticToken::Keyword("unsafe".to_string()));
        visit::visit_expr_unsafe(self, node);
    }

    fn visit_ident(&mut self, node: &'ast syn::Ident) {
        self.tokens
            .push(SemanticToken::Identifier(node.to_string()));
        visit::visit_ident(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if node.sig.unsafety.is_some() {
            self.tokens
                .push(SemanticToken::Keyword("unsafe".to_string()));
        }
        if let syn::ReturnType::Type(_, ty) = &node.sig.output {
            if let syn::Type::Path(type_path) = &**ty {
                self.tokens
                    .push(SemanticToken::Path(path_segments(&type_path.path)));
            }
        }
        visit::visit_item_fn(self, node);
    }

    fn visit_item_foreign_mod(&mut self, node: &'ast ItemForeignMod) {
        if node.unsafety.is_some() {
            self.tokens
                .push(SemanticToken::Keyword("unsafe".to_string()));
        }
        visit::visit_item_foreign_mod(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        self.tokens
            .push(SemanticToken::Module(node.ident.to_string()));
        visit::visit_item_mod(self, node);
    }

    fn visit_attribute(&mut self, node: &'ast Attribute) {
        self.tokens.push(SemanticToken::Attribute {
            path: path_segments(&node.path()),
            args: attribute_args(node),
        });
        visit::visit_attribute(self, node);
    }
}

pub(crate) fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect()
}

pub(crate) fn macro_name(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

pub(crate) fn expr_to_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(lit_str) => Some(lit_str.value()),
            syn::Lit::Int(lit_int) => Some(lit_int.base10_digits().to_string()),
            syn::Lit::Bool(lit_bool) => Some(lit_bool.value.to_string()),
            _ => None,
        },
        Expr::Path(expr_path) => expr_path.path.get_ident().map(|ident| ident.to_string()),
        Expr::Call(_) => Some("call".to_string()),
        _ => None,
    }
}

pub(crate) fn receiver_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(expr_path) => expr_path.path.get_ident().map(|ident| ident.to_string()),
        Expr::Reference(expr_ref) => receiver_name(&expr_ref.expr),
        _ => None,
    }
}

pub(crate) fn attribute_args(node: &Attribute) -> Vec<String> {
    match &node.meta {
        syn::Meta::List(list) => {
            let content = list.tokens.to_string();
            if content.is_empty() {
                Vec::new()
            } else {
                vec![content]
            }
        }
        _ => Vec::new(),
    }
}

use std::collections::HashMap;
use std::fmt;

use crate::infer::apply_subst_map_to_type;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar {
    pub id: u64,
}

impl TypeVar {
    pub fn new(id: u64) -> Self {
        TypeVar { id }
    }
}

impl fmt::Display for TypeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "T{}", self.id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Var(TypeVar),
    Int,
    Float,
    Bool,
    Char,
    Function(Box<Type>, Box<Type>),
    List(Box<Type>),
    Unit,
    Tuple(Vec<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Unit => write!(f, "()"),
            Type::Var(v) => write!(f, "{}", v),
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::Char => write!(f, "Char"),
            Type::Function(from, to) => match **from {
                Type::Function(_, _) => write!(f, "({}) -> {}", from, to),
                _ => write!(f, "{} -> {}", from, to),
            },
            Type::List(inner) => write!(f, "[{}]", inner),
            Type::Tuple(items) => {
                let inner = items
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(" * ");
                write!(f, "{{{}}}", inner)
            }
        }
    }
}

// Type scheme (for polymorphic types)
#[derive(Debug, Clone)]
pub struct TypeScheme {
    pub vars: Vec<u64>,
    pub typ: Type,
}

impl TypeScheme {
    pub fn new(vars: Vec<u64>, typ: Type) -> Self {
        TypeScheme { vars, typ }
    }

    pub fn monotype(typ: Type) -> Self {
        TypeScheme::new(vec![], typ)
    }

    pub fn free_vars(&self) -> std::collections::HashSet<u64> {
        // free vars of the body minus the bound vars of the scheme
        let mut fv = self.typ.free_vars();
        for v in &self.vars {
            fv.remove(v);
        }
        fv
    }
}

impl fmt::Display for TypeScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.vars.is_empty() {
            write!(f, "{}", self.typ)
        } else {
            let var_names: Vec<String> = self.vars.iter().map(|v| format!("{}", v)).collect();
            write!(f, "T{}. {}", var_names.join(" "), self.typ)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub scopes: Vec<HashMap<String, TypeScheme>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop().expect("Cannot exit global scope");
    }

    pub fn insert(&mut self, name: String, scheme: TypeScheme) -> Result<(), String> {
        let current = self.scopes.last_mut().unwrap();
        if name != "." && current.contains_key(&name) {
            return Err(format!(
                "Error! Variable '{}' already defined in this scope",
                name
            ));
        }
        current.insert(name, scheme);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<TypeScheme> {
        for scope in self.scopes.iter().rev() {
            if let Some(scheme) = scope.get(name) {
                return Some(scheme.clone());
            }
        }
        None
    }

    pub fn free_vars(&self) -> std::collections::HashSet<u64> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.values().flat_map(|scheme| scheme.free_vars()))
            .collect()
    }

    pub fn apply_substitution_map(&mut self, subst: &HashMap<u64, Type>) {
        for scope in &mut self.scopes {
            for ty in scope.values_mut() {
                ty.typ = apply_subst_map_to_type(subst, &ty.typ);
            }
        }
    }
}

impl Type {
    pub fn var_id(&self) -> Option<u64> {
        if let Type::Var(v) = self {
            Some(v.id)
        } else {
            None
        }
    }
    pub fn substitute(&self, subst: &HashMap<u64, Type>) -> Type {
        match self {
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit => self.clone(),
            Type::Var(v) => {
                if let Some(ty) = subst.get(&v.id) {
                    ty.clone()
                } else {
                    self.clone()
                }
            }
            Type::Function(from, to) => Type::Function(
                Box::new(from.substitute(subst)),
                Box::new(to.substitute(subst)),
            ),
            Type::List(inner) => Type::List(Box::new(inner.substitute(subst))),
            Type::Tuple(items) => Type::Tuple(
                items
                    .iter()
                    .map(|t| apply_subst_map_to_type(subst, t))
                    .collect(),
            ),
        }
    }

    pub fn free_vars(&self) -> std::collections::HashSet<u64> {
        let mut vars = std::collections::HashSet::new();
        self.collect_free_vars(&mut vars);
        vars
    }

    fn collect_free_vars(&self, vars: &mut std::collections::HashSet<u64>) {
        match self {
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit => {}
            Type::Var(v) => {
                vars.insert(v.id);
            }
            Type::Function(from, to) => {
                from.collect_free_vars(vars);
                to.collect_free_vars(vars);
            }
            Type::List(inner) => inner.collect_free_vars(vars),
            Type::Tuple(items) => {
                for it in items {
                    it.collect_free_vars(vars);
                }
            }
        }
    }
}

// Generalization and instantiation
pub fn generalize(env: &TypeEnv, typ: Type) -> TypeScheme {
    let env_vars = env.free_vars();
    let typ_vars = typ.free_vars();
    let vars: Vec<u64> = typ_vars.difference(&env_vars).cloned().collect();
    TypeScheme::new(vars, typ)
}

// Built-in function type signatures
pub fn create_builtin_environment(mut env: TypeEnv) -> (TypeEnv, u64) {
    // Local fresh-var generator
    let mut fresh_id: u64 = env.scopes.len() as u64;

    let mut fresh_var = || {
        let id = fresh_id;
        fresh_id += 1;
        Type::Var(TypeVar { id })
    };
    {
        let a: Type = fresh_var();
        let _ = env.insert(
            "set!".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::List(Box::new(a.clone()))),
                    Box::new(Type::Function(
                        Box::new(Type::Int),
                        Box::new(Type::Function(Box::new(a), Box::new(Type::Unit))),
                    )),
                ),
            ),
        );
    }
    {
        let a: Type = fresh_var();
        let _ = env.insert(
            "pop!".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(Box::new(Type::List(Box::new(a))), Box::new(Type::Unit)),
            ),
        );
    }
    {
        let a = fresh_var();
        let _ = env.insert(
            "length".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(Box::new(Type::List(Box::new(a))), Box::new(Type::Int)),
            ),
        );
    }
    {
        let a: Type = fresh_var();
        let _ = env.insert(
            "get".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::List(Box::new(a.clone()))),
                    Box::new(Type::Function(Box::new(Type::Int), Box::new(a))),
                ),
            ),
        );
    }
    {
        let a: Type = fresh_var();
        let _ = env.insert(
            "car".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(Box::new(Type::List(Box::new(a.clone()))), Box::new(a)),
            ),
        );
    }

    {
        let a: Type = fresh_var();
        let _ = env.insert(
            "cdr".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::List(Box::new(a.clone()))),
                    Box::new(Type::Function(
                        Box::new(Type::Int),
                        Box::new(Type::List(Box::new(a.clone()))),
                    )),
                ),
            ),
        );
    }
    {
        let a = fresh_var();
        let b = fresh_var();
        let _ = env.insert(
            "fst".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap(), b.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::Tuple(vec![a.clone(), b.clone()])),
                    Box::new(a.clone()),
                ),
            ),
        );
    }

    {
        let a = fresh_var();
        let b = fresh_var();
        let _ = env.insert(
            "snd".to_string(),
            TypeScheme::new(
                vec![a.var_id().unwrap(), b.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::Tuple(vec![a.clone(), b.clone()])),
                    Box::new(b.clone()),
                ),
            ),
        );
    }
    {
        let e = fresh_var();

        let _ = env.insert(
            "loop".to_string(),
            TypeScheme::new(
                vec![e.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::Int),
                    Box::new(Type::Function(
                        Box::new(Type::Int),
                        Box::new(Type::Function(
                            Box::new(Type::Function(Box::new(Type::Int), Box::new(e.clone()))),
                            Box::new(Type::Unit),
                        )),
                    )),
                ),
            ),
        );
    }
    {
        let e = fresh_var();

        let _ = env.insert(
            "loop-finish".to_string(),
            TypeScheme::new(
                vec![e.var_id().unwrap()],
                Type::Function(
                    Box::new(Type::Bool),
                    Box::new(Type::Function(Box::new(e.clone()), Box::new(Type::Unit))),
                ),
            ),
        );
    }
    let _ = env.insert(
        "+".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );
    let _ = env.insert(
        "+.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );
    let _ = env.insert(
        "+#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Char))),
        )),
    );

    let _ = env.insert(
        "-".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "-.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );

    let _ = env.insert(
        "-#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Char))),
        )),
    );

    let _ = env.insert(
        "*".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "*.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );

    let _ = env.insert(
        "*#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Char))),
        )),
    );

    let _ = env.insert(
        "/".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "/.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );

    let _ = env.insert(
        "/#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Char))),
        )),
    );
    let _ = env.insert(
        "/.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );
    let _ = env.insert(
        "mod".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );
    let _ = env.insert(
        "mod.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Float))),
        )),
    );

    // Comparison operations
    let _ = env.insert(
        "=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "=.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">=.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<=.".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Float),
            Box::new(Type::Function(Box::new(Type::Float), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert("true".to_string(), TypeScheme::monotype(Type::Bool));
    let _ = env.insert("false".to_string(), TypeScheme::monotype(Type::Bool));

    let _ = env.insert(
        "=?".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Bool),
            Box::new(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "=#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "<=#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">=".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        ">=#".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Char),
            Box::new(Type::Function(Box::new(Type::Char), Box::new(Type::Bool))),
        )),
    );

    // Logical operations
    let _ = env.insert(
        "and".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Bool),
            Box::new(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "or".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Bool),
            Box::new(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
        )),
    );

    let _ = env.insert(
        "not".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Bool), Box::new(Type::Bool))),
    );

    // Bitwise operations
    let _ = env.insert(
        "&".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "|".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "^".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        ">>".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "<<".to_string(),
        TypeScheme::monotype(Type::Function(
            Box::new(Type::Int),
            Box::new(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
        )),
    );

    let _ = env.insert(
        "Int->Float".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Int), Box::new(Type::Float))),
    );

    let _ = env.insert(
        "Float->Int".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Float), Box::new(Type::Int))),
    );

    let _ = env.insert(
        "~".to_string(),
        TypeScheme::monotype(Type::Function(Box::new(Type::Int), Box::new(Type::Int))),
    );

    (env, fresh_id) // return env with built-ins and also return next fresh ID
}

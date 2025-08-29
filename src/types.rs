use std::collections::HashMap;
use std::fmt;

// Type variables for type inference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar {
    id: u64,
}

impl TypeVar {
    pub fn new(id: u64) -> Self {
        TypeVar { id }
    }
}

impl fmt::Display for TypeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.id)
    }
}

// Types in the Hindley-Milner system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Var(TypeVar),
    Int,
    Bool,
    Function(Box<Type>, Box<Type>),
    List(Box<Type>),
    Unit,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Var(v) => write!(f, "{}", v),
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::Function(from, to) => match **from {
                Type::Function(_, _) => write!(f, "({}) -> {}", from, to),
                _ => write!(f, "{} -> {}", from, to),
            },
            Type::List(inner) => write!(f, "[{}]", inner),
            Type::Unit => write!(f, "()"),
        }
    }
}

// Type scheme (for polymorphic types)
#[derive(Debug, Clone)]
pub struct TypeScheme {
    pub vars: Vec<TypeVar>, // Quantified type variables
    pub typ: Type,
}

impl TypeScheme {
    pub fn new(vars: Vec<TypeVar>, typ: Type) -> Self {
        TypeScheme { vars, typ }
    }

    pub fn monotype(typ: Type) -> Self {
        TypeScheme::new(vec![], typ)
    }
    // original
    // pub fn instantiate(&self) -> Type {
    //     let mut subst = HashMap::new();
    //     for var in &self.vars {
    //         subst.insert(var.id, Type::Var(TypeVar::new(var.id + 1000))); // Offset to avoid conflicts
    //     }
    //     self.typ.substitute(&subst)
    // }
    pub fn instantiate(&self) -> Type {
        if self.vars.is_empty() {
            return self.typ.clone(); // don’t freshen monotypes
        }
        let mut subst = HashMap::new();
        for var in &self.vars {
            subst.insert(var.id, Type::Var(TypeVar::new(var.id + 1000)));
        }
        self.typ.substitute(&subst)
    }
}

impl fmt::Display for TypeScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.vars.is_empty() {
            write!(f, "{}", self.typ)
        } else {
            let var_names: Vec<String> = self.vars.iter().map(|v| format!("{}", v)).collect();
            write!(f, "∀{}. {}", var_names.join(" "), self.typ)
        }
    }
}

// Type environment for storing type assumptions
#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub bindings: HashMap<String, TypeScheme>,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            bindings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, scheme: TypeScheme) {
        self.bindings.insert(name, scheme);
    }

    pub fn get(&self, name: &str) -> Option<TypeScheme> {
        self.bindings.get(name).cloned()
    }
}

// Substitutions map type variables to types
#[derive(Debug, Clone)]
pub struct Substitution {
    map: HashMap<u64, Type>,
}

impl Substitution {
    pub fn new() -> Self {
        Substitution {
            map: HashMap::new(),
        }
    }

    pub fn empty() -> Self {
        Substitution::new()
    }

    pub fn insert(&mut self, var: u64, ty: Type) {
        self.map.insert(var, ty);
    }

    pub fn get(&self, var: &u64) -> Option<&Type> {
        self.map.get(var)
    }

    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => {
                if let Some(substituted) = self.get(&v.id) {
                    self.apply(substituted)
                } else {
                    ty.clone()
                }
            }
            Type::Function(from, to) => {
                Type::Function(Box::new(self.apply(from)), Box::new(self.apply(to)))
            }
            Type::List(inner) => Type::List(Box::new(self.apply(inner))),
            _ => ty.clone(),
        }
    }

    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = Substitution::new();

        // Apply other substitution to our types
        for (var, ty) in &self.map {
            result.insert(*var, other.apply(ty));
        }

        // Add other's mappings
        for (var, ty) in &other.map {
            if !result.map.contains_key(var) {
                result.insert(*var, ty.clone());
            }
        }

        result
    }
}

// Implementation for Type substitution
impl Type {
    pub fn substitute(&self, subst: &HashMap<u64, Type>) -> Type {
        match self {
            Type::Int | Type::Bool | Type::Unit => self.clone(),
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
        }
    }

    pub fn free_vars(&self) -> std::collections::HashSet<u64> {
        let mut vars = std::collections::HashSet::new();
        self.collect_free_vars(&mut vars);
        vars
    }

    fn collect_free_vars(&self, vars: &mut std::collections::HashSet<u64>) {
        match self {
            Type::Int | Type::Bool | Type::Unit => {}
            Type::Var(v) => {
                vars.insert(v.id);
            }
            Type::Function(from, to) => {
                from.collect_free_vars(vars);
                to.collect_free_vars(vars);
            }
            Type::List(inner) => inner.collect_free_vars(vars),
        }
    }
}

// Implementation for TypeScheme
impl TypeScheme {
    pub fn substitute(&self, subst: &HashMap<u64, Type>) -> TypeScheme {
        // Only substitute variables not bound by the scheme
        let bound_vars: std::collections::HashSet<_> = self.vars.iter().map(|v| v.id).collect();
        let filtered_subst: HashMap<u64, Type> = subst
            .iter()
            .filter(|(var, _)| !bound_vars.contains(var))
            .map(|(k, v)| (*k, v.clone()))
            .collect();

        TypeScheme::new(self.vars.clone(), self.typ.substitute(&filtered_subst))
    }

    pub fn free_vars(&self) -> std::collections::HashSet<u64> {
        let mut vars = self.typ.free_vars();
        for bound_var in &self.vars {
            vars.remove(&bound_var.id);
        }
        vars
    }
}

// Implementation for TypeEnv
impl TypeEnv {
    pub fn substitute(&self, subst: &HashMap<u64, Type>) -> TypeEnv {
        let mut new_env = TypeEnv::new();

        for (name, scheme) in &self.bindings {
            new_env
                .bindings
                .insert(name.clone(), scheme.substitute(subst));
        }

        new_env
    }

    pub fn free_vars(&self) -> std::collections::HashSet<u64> {
        let mut vars = std::collections::HashSet::new();

        for scheme in self.bindings.values() {
            vars.extend(scheme.free_vars());
        }

        vars
    }
}

// Unification algorithm
pub fn unify(ty1: &Type, ty2: &Type) -> Result<Substitution, String> {
    match (ty1, ty2) {
        (Type::Int, Type::Int) | (Type::Bool, Type::Bool) | (Type::Unit, Type::Unit) => {
            Ok(Substitution::empty())
        }

        (Type::Var(v1), Type::Var(v2)) if v1.id == v2.id => Ok(Substitution::empty()),

        (Type::Var(v), ty) | (ty, Type::Var(v)) => {
            if occurs_in(v, ty) {
                Err(format!("Occurs check failed: {} occurs in {}", v, ty))
            } else {
                let mut sub = Substitution::empty();
                sub.insert(v.id, ty.clone());
                Ok(sub)
            }
        }

        (Type::Function(from1, to1), Type::Function(from2, to2)) => {
            let sub1 = unify(from1, from2)?;
            let sub2 = unify(&sub1.apply(to1), &sub1.apply(to2))?;
            Ok(sub1.compose(&sub2))
        }

        (Type::List(inner1), Type::List(inner2)) => unify(inner1, inner2),

        _ => Err(format!("Cannot unify {} with {}", ty1, ty2)),
    }
}

// Check if a type variable occurs in a type
fn occurs_in(var: &TypeVar, ty: &Type) -> bool {
    match ty {
        Type::Var(v) => v.id == var.id,
        Type::Function(from, to) => occurs_in(var, from) || occurs_in(var, to),
        Type::List(inner) => occurs_in(var, inner),
        _ => false,
    }
}

// Generalization and instantiation
pub fn generalize(env: &TypeEnv, typ: Type) -> TypeScheme {
    let env_vars = env.free_vars();
    let typ_vars = typ.free_vars();

    let vars: Vec<TypeVar> = typ_vars
        .difference(&env_vars)
        .map(|&id| TypeVar::new(id))
        .collect();

    TypeScheme::new(vars, typ)
}

pub fn instantiate(scheme: &TypeScheme) -> Type {
    scheme.instantiate()
}

pub fn solve_constraints(constraints: Vec<(Type, Type)>) -> Result<Substitution, String> {
    let mut subst = Substitution::empty();
    let mut worklist = constraints;

    while let Some((t1, t2)) = worklist.pop() {
        // Apply current substitution before unifying
        let t1_applied = subst.apply(&t1);
        let t2_applied = subst.apply(&t2);

        let s = unify(&t1_applied, &t2_applied)?;
        subst = subst.compose(&s);

        // Apply the new substitution to all remaining constraints
        worklist = worklist
            .into_iter()
            .map(|(a, b)| (s.apply(&a), s.apply(&b)))
            .collect();
    }

    Ok(subst)
}

use std::collections::HashMap;
use std::fmt;

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
        write!(f, "t{}", self.id)
    }
}

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
            Type::Unit => write!(f, "()"),
            Type::Var(v) => write!(f, "{}", v),
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::Function(from, to) => match **from {
                Type::Function(_, _) => write!(f, "({}) -> {}", from, to),
                _ => write!(f, "{} -> {}", from, to),
            },
            Type::List(inner) => write!(f, "[{}]", inner),
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
    pub fn substitute(&self, subst: &HashMap<u64, Type>) -> TypeScheme {
        // Only substitute variables not bound by the scheme
        let bound_vars: std::collections::HashSet<_> = self.vars.iter().collect();
        let filtered_subst: HashMap<u64, Type> = subst
            .iter()
            .filter(|(var, _)| !bound_vars.contains(var))
            .map(|(k, v)| (*k, v.clone()))
            .collect();

        TypeScheme::new(self.vars.clone(), self.typ.substitute(&filtered_subst))
    }

    pub fn new(vars: Vec<u64>, typ: Type) -> Self {
        TypeScheme { vars, typ }
    }

    pub fn monotype(typ: Type) -> Self {
        TypeScheme::new(vec![], typ)
    }

    pub fn apply(&self, sub: &Substitution) -> TypeScheme {
        // filter out mappings for bound vars
        let filtered = sub.without(&self.vars);
        TypeScheme::new(self.vars.clone(), filtered.apply(&self.typ))
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
            write!(f, "∀{}. {}", var_names.join(" "), self.typ)
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
            return Err(format!("Variable '{}' already defined in this scope", name));
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

    pub fn apply_in_place(&mut self, subst: &Substitution) {
        for scope in &mut self.scopes {
            for scheme in scope.values_mut() {
                *scheme = scheme.apply(subst);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Substitution {
    pub map: HashMap<u64, Type>,
}

impl Substitution {
    pub fn new() -> Self {
        Substitution {
            map: HashMap::new(),
        }
    }
    pub fn without(&self, bound: &[u64]) -> Substitution {
        let mut new_map = self.map.clone();
        for b in bound {
            new_map.remove(b);
        }
        Substitution { map: new_map }
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
pub fn occurs_in(var: &TypeVar, ty: &Type) -> bool {
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
    let vars: Vec<u64> = typ_vars.difference(&env_vars).cloned().collect();
    TypeScheme::new(vars, typ)
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

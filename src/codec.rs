//! Tree <-> .rbxm bytes using rbx_dom_weak + rbx_binary.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, bail, Context, Result};
use rbx_dom_weak::{types::Ref, InstanceBuilder, WeakDom};
use rbx_reflection::{DataType, PropertyKind};
use rbx_types::VariantType;

use crate::convert::{from_variant, parse_type, to_variant};
use crate::wire::{InstanceDesc, Tree};

pub struct Limits {
    pub max_instances: usize,
    pub max_depth: usize,
}

/// Finds the data type the reflection database expects for `class.prop`,
/// searching superclasses. Aliases are followed.
pub fn expected_type(class: &str, prop: &str) -> Option<VariantType> {
    expected(class, prop).map(|(t, _)| t)
}

/// Like `expected_type`, but also returns the enum name for enum-typed properties.
pub fn expected(class: &str, prop: &str) -> Option<(VariantType, Option<&'static str>)> {
    let db = rbx_reflection_database::get().ok()?;
    let mut cur = db.classes.get(class)?;
    loop {
        if let Some(desc) = cur.properties.get(prop) {
            if let PropertyKind::Alias { alias_for } = &desc.kind {
                return expected(class, alias_for);
            }
            return Some(match desc.data_type {
                DataType::Value(t) => (t, None),
                DataType::Enum(e) => (VariantType::Enum, Some(e)),
                _ => return None,
            });
        }
        cur = db.classes.get(cur.superclass?)?;
    }
}

pub fn encode(tree: &Tree, limits: &Limits) -> Result<Vec<u8>> {
    let n = tree.instances.len();
    if n == 0 {
        bail!("no instances");
    }
    if n > limits.max_instances {
        bail!("too many instances ({n} > {})", limits.max_instances);
    }

    // Pass 1: assign a referent to every id.
    let mut refs: HashMap<String, Ref> = HashMap::with_capacity(n);
    for inst in &tree.instances {
        if refs.insert(inst.id.clone(), Ref::new()).is_some() {
            bail!("duplicate instance id {:?}", inst.id);
        }
    }

    // children[parent id] = indexes; roots = no parent (or unknown parent).
    let mut children: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();
    for (i, inst) in tree.instances.iter().enumerate() {
        match inst.parent.as_deref() {
            Some(p) if refs.contains_key(p) => children.entry(p).or_default().push(i),
            _ => roots.push(i),
        }
    }

    // Pass 2: build instances and insert them parent-first (iterative, depth-limited).
    let mut dom = WeakDom::new(InstanceBuilder::new("DataModel"));
    let dom_root = dom.root_ref();
    let mut top: Vec<Ref> = Vec::with_capacity(roots.len());
    let mut inserted = 0usize;

    let mut stack: Vec<(usize, Ref, usize)> = roots.iter().rev().map(|&i| (i, dom_root, 1)).collect();
    while let Some((idx, parent_ref, depth)) = stack.pop() {
        if depth > limits.max_depth {
            bail!("tree deeper than {}", limits.max_depth);
        }
        let inst = &tree.instances[idx];
        let builder = build_instance(inst, &refs).with_context(|| format!("instance {:?} ({})", inst.id, inst.class))?;
        let my_ref = dom.insert(parent_ref, builder);
        inserted += 1;
        if parent_ref == dom_root {
            top.push(my_ref);
        }
        if let Some(kids) = children.get(inst.id.as_str()) {
            for &k in kids.iter().rev() {
                stack.push((k, my_ref, depth + 1));
            }
        }
    }
    if inserted != n {
        bail!("{} instance(s) are unreachable (parent cycle?)", n - inserted);
    }

    let mut out = Vec::new();
    rbx_binary::to_writer(&mut out, &dom, &top).map_err(|e| anyhow!("rbx_binary: {e}"))?;
    Ok(out)
}

fn build_instance(inst: &InstanceDesc, refs: &HashMap<String, Ref>) -> Result<InstanceBuilder> {
    let mut b = InstanceBuilder::new(inst.class.as_str())
        .with_referent(refs[&inst.id])
        .with_name(inst.name.clone().unwrap_or_else(|| inst.class.clone()));

    for (prop, wire) in &inst.properties {
        if prop == "Name" || prop == "Parent" || prop == "ClassName" {
            continue;
        }
        // The reflection database is authoritative; fall back to the client's tag for unknown properties.
        let ty = match expected_type(&inst.class, prop) {
            Some(t) => t,
            None => parse_type(&wire.t).ok_or_else(|| anyhow!("property {prop}: unknown type {:?}", wire.t))?,
        };
        let value = to_variant(ty, &wire.v, refs).with_context(|| format!("property {prop}"))?;
        b.add_property(prop.as_str(), value);
    }
    Ok(b)
}

pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Tree> {
    let dom = rbx_binary::from_reader(bytes).map_err(|e| anyhow!("rbx_binary: {e}"))?;

    // Assign ids in depth-first (parent-first) order.
    let mut order: Vec<Ref> = Vec::new();
    let mut stack: Vec<(Ref, usize)> = dom.root().children().iter().rev().map(|&r| (r, 1)).collect();
    while let Some((r, depth)) = stack.pop() {
        if depth > limits.max_depth {
            bail!("tree deeper than {}", limits.max_depth);
        }
        order.push(r);
        if order.len() > limits.max_instances {
            bail!("too many instances (> {})", limits.max_instances);
        }
        let inst = dom.get_by_ref(r).ok_or_else(|| anyhow!("dangling referent"))?;
        for &c in inst.children().iter().rev() {
            stack.push((c, depth + 1));
        }
    }
    let ids: HashMap<Ref, String> = order.iter().enumerate().map(|(i, r)| (*r, (i + 1).to_string())).collect();
    let top: HashSet<Ref> = dom.root().children().iter().copied().collect();

    let mut instances = Vec::with_capacity(order.len());
    for r in &order {
        let inst = dom.get_by_ref(*r).unwrap();
        let mut properties = std::collections::BTreeMap::new();
        for (name, value) in inst.properties.iter() {
            if let Some(mut w) = from_variant(value, &ids) {
                if w.t == "Enum" {
                    w.e = expected(&inst.class, name).and_then(|(_, e)| e).map(str::to_owned);
                }
                properties.insert(name.to_string(), w);
            }
        }
        instances.push(InstanceDesc {
            id: ids[r].clone(),
            class: inst.class.to_string(),
            name: Some(inst.name.clone()),
            parent: if top.contains(r) { None } else { Some(ids[&inst.parent()].clone()) },
            properties,
        });
    }
    Ok(Tree { instances })
}

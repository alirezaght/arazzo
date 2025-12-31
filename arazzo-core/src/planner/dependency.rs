use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::planner::model::DependencyGraph;
use crate::types::Workflow;

pub(crate) fn build_step_dependency_graph(
    workflow: &Workflow,
    deps: &BTreeMap<String, BTreeSet<String>>,
) -> Result<DependencyGraph, String> {
    let mut depends_on: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let step_ids: BTreeSet<String> = workflow.steps.iter().map(|s| s.step_id.clone()).collect();

    for step_id in &step_ids {
        let mut d = deps
            .get(step_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|x| step_ids.contains(x))
            .collect::<Vec<_>>();
        d.sort();
        depends_on.insert(step_id.clone(), d);
    }

    let topo_order = topo_sort(&step_ids, &depends_on)?;
    let levels = compute_levels(&topo_order, &depends_on);

    Ok(DependencyGraph {
        depends_on,
        levels,
        topo_order,
    })
}

fn topo_sort(
    nodes: &BTreeSet<String>,
    depends_on: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut indeg: BTreeMap<String, usize> = nodes.iter().map(|n| (n.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (n, deps) in depends_on {
        for d in deps {
            if !nodes.contains(d) {
                continue;
            }
            *indeg.get_mut(n).unwrap() += 1;
            outgoing.entry(d.clone()).or_default().push(n.clone());
        }
    }

    for v in outgoing.values_mut() {
        v.sort();
    }

    let mut q = VecDeque::new();
    for n in nodes {
        if indeg[n] == 0 {
            q.push_back(n.clone());
        }
    }

    let mut out = Vec::with_capacity(nodes.len());
    while let Some(n) = q.pop_front() {
        out.push(n.clone());
        if let Some(nexts) = outgoing.get(&n) {
            for m in nexts {
                let e = indeg.get_mut(m).unwrap();
                *e -= 1;
                if *e == 0 {
                    q.push_back(m.clone());
                }
            }
        }
    }

    if out.len() != nodes.len() {
        return Err("cycle detected in step dependency graph".to_string());
    }
    Ok(out)
}

fn compute_levels(topo: &[String], depends_on: &BTreeMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut level: BTreeMap<String, usize> = BTreeMap::new();
    for node in topo {
        let deps = depends_on.get(node).map(|v| v.as_slice()).unwrap_or(&[]);
        let l = deps
            .iter()
            .filter_map(|d| level.get(d).copied())
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        level.insert(node.clone(), l);
    }

    let max_level = level.values().copied().max().unwrap_or(0);
    let mut levels = vec![Vec::<String>::new(); max_level + 1];
    for node in topo {
        let l = level[node];
        levels[l].push(node.clone());
    }
    levels
}

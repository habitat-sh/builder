// Copyright (c) 2020 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{collections::HashMap,
          fs::File,
          io::{Result,
               Write},
          path::Path};

use crate::hab_core::package::{Identifiable,
                               PackageIdent};

use crate::build_ordering::PackageBuild;

// This struct is really bad
#[derive(Default)]
struct BuildGraph {
    depends:    HashMap<PackageIdent, Vec<PackageIdent>>,
    descendant: HashMap<PackageIdent, PackageIdent>, // if
}

// impl BuildGraph {
//     pub fn add(p: &PackageBuild) {
//         let dep_flatten: Vector<PackageIdent> = p
//             .bt_deps
//             .iter()
//             .chain(package.rt_deps.iter())
//             .map(|d| d.clone())
//             .collect();
//         depends.insert(p.ident.clone(), dep_flatten)
//     }

//     pub fn depends_on(start: &PackageIdent, candidate: &PackageIdent) -> bool {
//         let mut seen = HashSet::<PackageIdent>::new();
//         let mut worklist : VecDeque<PackageIdent> = VecDeque::new();

//         for dep in depends[Start] {
//             worklist.push_back(pred);
//         }

//         while !worklist.is_empty() {
//             while !worklist.is_empty() {
//             visits += 1;

//             let node_index = worklist.pop_front().unwrap();

//         }

//     }
// }

// Takes a build manifest and an estimator of build time and builds an optimal schedule
//
static COMPLETE: &'static str = "complete";

pub fn emit_z3(workers: usize,
               packages: &[PackageBuild],
               times: &HashMap<PackageIdent, usize>,
               filename: &str)
               -> Result<()> {
    let path = Path::new(filename);
    let do_worker = workers > 1;

    let mut file = File::create(&path).unwrap();

    // Emit headers
    writeln!(&mut file, "; Multiple workers {} ({})", do_worker, workers)?;

    // From: https://rise4fun.com/Z3/tutorial/optimization
    //
    // For arithmetical constraints that only have differences between variables, known as
    // difference logic, Z3 furthermore contains alternative decision procedures tuned for this
    // domain. These have to be configured explicitly. There is a choice between a solver tuned
    // for sparse constraints (where the ratio of variables is high compared to number of
    // inequalities) and a solver tuned for dense constraints.
    writeln!(
             &mut file,
             "(set-logic QF_IDL)
(set-option :smt.arith.solver 1) ; enables difference logic solver for sparse constraints
(set-option :smt.arith.solver 3) ; enables difference logic solver for dense constraints
"
    )?;

    // Phase 1: create all needed variables; each package has a start time and an assigned worker
    writeln!(&mut file, "; declare variables")?;
    create_integer_var(&mut file, COMPLETE)?;

    for package in packages {
        let start_time = ident_to_start(&package.ident);
        create_integer_var(&mut file, &start_time)?;
        if do_worker {
            let worker_assignment = ident_to_worker(&package.ident);
            create_integer_var(&mut file, &worker_assignment)?;
        }
    }

    // Phase 2: Declare worker and basic time constraints
    writeln!(&mut file, ";\n;\n; basic constraints for time and worker")?;
    for package in packages {
        if do_worker {
            // bound worker
            let worker_assignment = ident_to_worker(&package.ident);
            writeln!(&mut file,
                     "(assert (and (>= {} 0) (>= {} {})))",
                     &worker_assignment,
                     (workers - 1),
                     &worker_assignment)?;
        }

        // can't start too early
        let start_time = ident_to_start(&package.ident);
        writeln!(&mut file, "(assert (>= {} 0))", &start_time)?;

        // bound completion
        writeln!(&mut file,
                 "(assert (>= (- {} {}) {}))",
                 COMPLETE,
                 start_time,
                 duration(&package.ident, times))?;
    }

    // Phase 3: create ordering constraints
    writeln!(&mut file, ";\n;\n; ordering constraints for packages")?;
    for package in packages {
        writeln!(&mut file, "; package {}", package.ident)?;
        let package_start = ident_to_start(&package.ident);
        for dep in package.bt_deps.iter().chain(package.rt_deps.iter()) {
            // ignore packages that aren't part of the rebuild
            if dep.release().unwrap().starts_with("N") {
                let dep_start = ident_to_start(&dep);
                let dep_duration = duration(dep, times);
                // constrain to start after our dep has finished
                writeln!(&mut file,
                         "(assert (>= (- {} {}) {}))",
                         package_start, dep_start, dep_duration)?
            }
        }
    }
    // Phase 4: create non-conflict constraints (n^2, maybe need to prune?)
    //
    // The relationship between packages forms a dag; pruning can be
    // done as long as one is an ancestor of the other.
    //
    //
    // This total_work_units
    let mut total_work_units = 0;
    writeln!(&mut file, "\n;\n; resource conflict for packages")?;
    let mut res_conflict_count = 0;
    for i in 0..packages.len() {
        let p1_start = ident_to_start(&packages[i].ident);
        let p1_worker = ident_to_worker(&packages[i].ident);
        let p1_duration = duration(&packages[i].ident, times);
        total_work_units += p1_duration;

        // Constrain against conflicts. This is symmetric, so we only have to do the lower triangle
        // TODO: Prune redundant constraings because of ancestor relationships
        for j in 0..i {
            let p2_start = ident_to_start(&packages[j].ident);
            let p2_duration = duration(&packages[j].ident, times);
            // do not overlap
            res_conflict_count += 1;
            let conflict_constraint =
                format!("(or (>= (- {} {}) {}) (>= (- {} {}) {}))",
                        p1_start, p2_start, p2_duration, p2_start, p1_start, p1_duration);
            if do_worker {
                let p2_worker = ident_to_worker(&packages[j].ident);
                writeln!(&mut file,
                         "(assert (or (!= {} {}) {}))",
                         p1_worker, p2_worker, conflict_constraint,)?;
            } else {
                writeln!(&mut file, "(assert {})", conflict_constraint)?;
            }
        }
    }

    // Phase 5a: constrain objective
    // The problem isn't as tightly specified as we like, so it spends time searching for
    // impossible solutions. We know that if we use 100% of our capacity it still is going to take
    // a minimum amount of time, so give a hint.
    // Also, the total_work_units is the maximum amount of time possible, so we might as well
    // hint about that
    let min_completion = if do_worker {
        ((total_work_units as f64) / (workers as f64)).ceil() as usize
    } else {
        total_work_units
    };
    writeln!(&mut file,
             "; {} total work units, {} min_completion",
             total_work_units, min_completion)?;
    writeln!(&mut file, "(assert (>= {} {}))", COMPLETE, min_completion)?;
    writeln!(&mut file, "(assert (>= {} {}))", total_work_units, COMPLETE)?;

    // Phase 5: add objective; (min of all completion times)
    writeln!(&mut file,
             ";\n;\n; Packages {} Resource Conflict constraints {}",
             packages.len(),
             res_conflict_count)?;
    writeln!(
             &mut file,
             "
(minimize {})
(check-sat)
(get-model)
(get-stats)
;(get-objectives)",
             COMPLETE
    )?;

    Ok(())
}

fn create_integer_var(fd: &mut File, name: &str) -> Result<()> {
    let decl = format!("(declare-fun {} () Int)", name);
    writeln!(fd, "{}", decl)
}

fn ident_to_worker(ident: &PackageIdent) -> String { ident_to_var("W", &ident) }
fn ident_to_start(ident: &PackageIdent) -> String { ident_to_var("S", &ident) }

fn ident_to_var(prefix: &str, ident: &PackageIdent) -> String {
    let release = ident.release().unwrap().replace("-", "_");

    format!("{}_{}_{}_{}", prefix, ident.origin, ident.name, release)
}

fn duration(ident: &PackageIdent, data: &HashMap<PackageIdent, usize>) -> usize {
    // todo use shortname
    match data.get(ident) {
        Some(d) => *d,
        None => 1,
    }
}

//! The partition rules: a list must name the dimension its elements share.

use std::collections::HashSet;

use super::*;

pub(crate) fn check_partition(
    decl: &Decl,
    decls: &[Decl],
    by_name: &std::collections::HashMap<&str, &Decl>,
    out: &mut Vec<Diagnostic>,
) {
    let elements = list_field(decl, "elements");
    if elements.is_empty() {
        sem(
            decl,
            "E-PARTITION-EMPTY",
            format!(
                "partition {} names no element, so it partitions nothing",
                decl.name
            ),
            Some("elements"),
            out,
        );
        return;
    }
    let mut seen: HashSet<String> = HashSet::new();
    for name in &elements {
        if !seen.insert(name.clone()) {
            sem(
                decl,
                "E-PARTITION-DUPLICATE",
                format!("partition {} lists {name} twice", decl.name),
                Some("elements"),
                out,
            );
        }
    }

    // the dimension's extent is what the list can be checked against: a concept already
    // records its positive examples, so closedness is a claim about a set we can see
    let dimension = id_field(decl, "dimension");
    let closure = id_field(decl, "closure").unwrap_or("open");
    let unknown = closure == "unknown";
    let referenced = decls.iter().any(|other| {
        (other.kind == "proposition" || other.kind == "warrant")
            && list_field(other, "from")
                .iter()
                .any(|name| name == &decl.name)
    });

    // naming a dimension and admitting you have none are two different claims, and exactly
    // one of them is true: saying both is the lie, saying neither is the silence
    if dimension.is_none() != unknown {
        let message = if dimension.is_none() {
            format!(
                "partition {} names no dimension, so it must be declared unknown rather than {closure}",
                decl.name
            )
        } else {
            format!(
                "partition {} names a dimension, so it cannot be unknown",
                decl.name
            )
        };
        sem(decl, "E-PARTITION-CLOSURE", message, Some("closure"), out);
    }

    let Some(dimension) = dimension else {
        // a pile, not a partition: it may be written, and it may not be used
        if list_field(decl, "closes_with").is_empty() {
            sem(
                decl,
                "E-PARTITION-UNKNOWN-NO-EXIT",
                format!(
                    "partition {} has no dimension and no question that would find one",
                    decl.name
                ),
                Some("closes_with"),
                out,
            );
        } else if referenced {
            sem(
                decl,
                "E-PARTITION-UNKNOWN-USED",
                format!(
                    "partition {} has no dimension, so it cannot discharge a premise",
                    decl.name
                ),
                Some("dimension"),
                out,
            );
        } else {
            sem(
                decl,
                "W-PARTITION-UNKNOWN",
                format!(
                    "partition {} is a pile with no type yet; the question it names is how the type will be found",
                    decl.name
                ),
                Some("closure"),
                out,
            );
        }
        return;
    };

    let target = by_name.get(dimension).copied();
    let resolved = target.filter(|target| target.kind == "concept");
    // E-UNDEFINED (a name nothing declares) and E-KIND (a dimension that is not a concept) have
    // already reported the only thing that is known.  Comparing elements against an extent that
    // does not exist would claim they are "not an instance" of something that is not there.
    let extent = resolved
        .map(|target| list_field(target, "grounded_by"))
        .unwrap_or_default();
    if let Some(concept) = resolved {
        for name in &elements {
            if !extent.contains(name) {
                sem(
                    decl,
                    "E-PARTITION-ELEMENT",
                    format!(
                        "{name} is not an instance of {dimension}, which is what the list claims"
                    ),
                    Some("elements"),
                    out,
                );
            }
        }

        // a named dimension is only a type if its own examples can be separated; if they
        // collide, the list leans on a distinction that was never recorded
        let has_negative = !list_field(concept, "against").is_empty();
        if has_negative {
            let program = crate::program::Program::new(decls);
            if crate::induce::induce_concept(&program, concept)
                .clause
                .is_none()
            {
                sem(
                    decl,
                    "W-PARTITION-UNLEARNED",
                    format!(
                        "partition {} leans on {dimension}, whose examples no recorded attribute separates",
                        decl.name
                    ),
                    Some("dimension"),
                    out,
                );
            }
        }
    }

    if closure == "closed" {
        if list_field(decl, "basis").is_empty() {
            sem(
                decl,
                "W-PARTITION-CLOSED-UNJUSTIFIED",
                format!(
                    "partition {} claims to be closed without a reason",
                    decl.name
                ),
                Some("closure"),
                out,
            );
        }
        if let Some(target) = resolved {
            for name in &extent {
                if !elements.contains(name) {
                    sem(
                        decl,
                        "E-PARTITION-CLOSED-CONTRADICTED",
                        format!("{dimension} also holds of {name}, which the closed list omits"),
                        Some("elements"),
                        out,
                    );
                }
            }
            // the attributes imply more instances than the concept lists; a closed list that
            // omits one is contradicted by its own recorded evidence, not only by the author
            let program = crate::program::Program::new(decls);
            let positives: Vec<&Decl> = crate::program::pos(target)
                .iter()
                .filter_map(|name| program.get(name))
                .collect();
            let pairs = crate::fca::intent(&program, &positives);
            if !pairs.is_empty() {
                for fact in crate::fca::extent(&program, &pairs) {
                    if !elements.contains(&fact.name) && !extent.contains(&fact.name) {
                        sem(
                            decl,
                            "E-PARTITION-CLOSED-CONTRADICTED",
                            format!(
                                "the recorded attributes also imply {}, which the closed list omits",
                                fact.name
                            ),
                            Some("elements"),
                            out,
                        );
                    }
                }
            }
        }
    } else if closure == "open" && referenced {
        // an open list used as a premise asserts in use what it denied in form
        sem(
            decl,
            "W-PARTITION-OPEN-USED",
            format!(
                "partition {} is open but used as a premise; the conclusion rests on a list known to be incomplete",
                decl.name
            ),
            Some("closure"),
            out,
        );
    }
}

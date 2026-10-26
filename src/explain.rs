//! What each diagnostic means, in the form `--explain` prints.
//!
//! A code without an explanation is a code nobody can act on, so the table is complete and a
//! test holds it to every code that appears in the corpus.

pub const CODES: &[&str] = &[
    "E-COLLAPSE",
    "E-CONFIDENCE",
    "E-CONTRADICTION",
    "E-CYCLE",
    "E-DUP-ID",
    "E-EMPTY",
    "E-EMPTY-DOCUMENT",
    "E-FALSE-BLINDSPOT",
    "E-FIELD",
    "E-HIERARCHY",
    "E-INCOMMENSURABLE",
    "E-KIND",
    "E-KIND-UNKNOWN",
    "E-LAYER-LEAK",
    "E-MISSING",
    "E-NO-ANSWERS",
    "E-NO-ASSUMPTION",
    "E-NO-EVIDENCE",
    "E-NO-PREMISE",
    "E-NO-SIGNATURE",
    "E-NON-DISCRIMINATING",
    "E-NOTE-REF",
    "E-PARTITION-CLOSED-CONTRADICTED",
    "E-PARTITION-CLOSURE",
    "E-PARTITION-DUPLICATE",
    "E-PARTITION-ELEMENT",
    "E-PARTITION-EMPTY",
    "E-PARTITION-UNKNOWN-NO-EXIT",
    "E-PARTITION-UNKNOWN-USED",
    "E-SYNTAX",
    "E-TENSION",
    "E-TYPE",
    "E-UNCOVERED",
    "E-UNDEFINED",
    "E-UNFALSIFIABLE",
    "E-UNGROUNDED",
    "E-VACUOUS",
    "E-WARRANT-MODE",
    "E-WITNESS",
    "W-BY-MISMATCH",
    "W-COARSE",
    "W-CONDITIONAL",
    "W-CONFIDENCE-ASSERTED",
    "W-CONFIDENCE-INFLATED",
    "W-CONFIDENCE-MODEL",
    "W-EXAMPLE-SHAPE",
    "W-EXPOSE-FOREIGN",
    "W-HOLE",
    "W-LAW-UNCHECKED",
    "W-NO-EXIT",
    "W-OPEN",
    "W-OVERCLAIM",
    "W-PARTITION-CLOSED-UNJUSTIFIED",
    "W-PARTITION-OPEN-USED",
    "W-PARTITION-UNKNOWN",
    "W-PARTITION-UNLEARNED",
    "W-UNCONFIRMED",
    "W-UNTESTED",
    "W-UNWARRANTED",
    "W-UNWARRANTED-BASIS",
];

pub fn explain(code: &str) -> Option<&'static str> {
    Some(match code {
        "E-COLLAPSE" => "Two concepts have the same extent: the same positives and the same negatives.\n\
            A concept with no positive example cannot collide.\n\
            Fix: merge them, or give one an example the other lacks.",
        "E-CONFIDENCE" => "A confidence is outside 0..1. The number is a fraction of certainty, not a score.\n\
            Fix: bring it into range, or drop it.",
        "E-CONTRADICTION" => "A fact is listed both as a positive and as a negative example of one concept.\n\
            Fix: decide which side it belongs to; an example cannot show both.",
        "E-CYCLE" => "Declarations justify each other in a circle, so nothing in the circle is grounded.\n\
            Fix: cut one edge and ground the rest in a fact.",
        "E-DUP-ID" => "One name is declared twice. Names are global within a project.\n\
            Fix: give one a distinct name, or delete it if it is a duplicate.",
        "E-EMPTY" => "A required string is present but blank.\n\
            Fix: write the text, or turn the field into a hole with `?` if it is not known yet.",
        "E-EMPTY-DOCUMENT" => "The document declares nothing, so there is nothing to check.\n\
            An empty answer is not a passing answer.\n\
            Fix: write at least one declaration.",
        "E-FALSE-BLINDSPOT" => "The claimed blind spot is not blind: something already separates the witness pair.\n\
            Fix: name the concept that separates them, or choose a pair nothing separates.",
        "E-FIELD" => "A declaration carries a field its kind does not have.\n\
            Fix: remove it, or check the kind's field list.",
        "E-HIERARCHY" => "A concept is declared under a parent, but its examples fall outside the parent's extent.\n\
            Ext is only a functor when every child sits inside its parent.\n\
            Fix: add the missing examples to the parent, or drop the parent link.",
        "E-INCOMMENSURABLE" => "A value lacks a unit or a threshold, so it cannot be compared with anything.\n\
            Fix: name the unit and the bound, or leave it as a hole on purpose.",
        "E-KIND" => "A reference points at a declaration of the wrong kind.\n\
            Fix: point it at the kind the field expects, or change the field's target.",
        "E-KIND-UNKNOWN" => "The declaration kind is not one the language has.\n\
            Fix: use one of the twelve kinds, or add the kind to the schema.",
        "E-LAYER-LEAK" => "A declaration reaches across layers into a name the target layer does not expose.\n\
            Fix: expose it, or remove the reference.",
        "E-MISSING" => "A required field is absent.\n\
            Fix: add it. If the value is not known yet, write `?` — that is a hole, not an error.",
        "E-NO-ANSWERS" => "A question admits fewer than two answers, so it asks nothing.\n\
            Fix: give the answer set at least two possibilities.",
        "E-NO-ASSUMPTION" => "A hypothesis assumes nothing, so nothing could refute it.\n\
            Fix: name what it rests on.",
        "E-NO-EVIDENCE" => "A hypothesis carries a verdict, but no fact decided it.\n\
            Fix: point verdict_by at the fact that settled it, or remove the verdict.",
        "E-NO-PREMISE" => "A warrant licenses an inference from nothing.\n\
            Fix: name the premises the rule consumes.",
        "E-NO-SIGNATURE" => "A blind spot names no signature, so it is not relative to anything.\n\
            Fix: name the concepts that fail to express the distinction.",
        "E-NON-DISCRIMINATING" => "A question does not separate two hypotheses; it earns its place only by splitting a live disagreement.\n\
            Fix: name two hypotheses, or drop the question.",
        "E-NOTE-REF" => "A commitment leans on a note. Notes are not commitments and cannot carry weight.\n\
            Fix: promote the note to a typed declaration, or stop depending on it.",
        "E-PARTITION-CLOSED-CONTRADICTED" => "A list claims to be closed, but the dimension's extent or its attributes imply an element the list omits.\n\
            Fix: add the element, or say the list is open.",
        "E-PARTITION-CLOSURE" => "Closure and dimension disagree: a list with no dimension must be declared unknown, and a list with one cannot be.\n\
            Fix: name the dimension, or declare the pile unknown.",
        "E-PARTITION-DUPLICATE" => "The same element is listed twice.\n\
            Fix: remove the repetition.",
        "E-PARTITION-ELEMENT" => "An element is not an instance of the list's dimension.\n\
            Fix: add it to the dimension's extent, or remove it from the list.",
        "E-PARTITION-EMPTY" => "A partition names no element, so it partitions nothing.\n\
            Fix: list the elements, or delete the declaration.",
        "E-PARTITION-UNKNOWN-NO-EXIT" => "A pile with no dimension names no question that would supply one, so it can never be typed.\n\
            Fix: add closes_with pointing at the question.",
        "E-PARTITION-UNKNOWN-USED" => "A pile with no dimension is used as a premise. A hole cannot discharge anything.\n\
            Fix: give it a dimension, or stop using it as support.",
        "E-SYNTAX" => "The document does not parse: a string is unterminated, a bracket is missing, or a value is not a value.\n\
            Fix: the caret points at the first token that could not start what the grammar expected.",
        "E-TENSION" => "Two values constrain the same metric in opposite directions and neither has a priority.\n\
            Fix: give one a higher priority, or split the metric by phase.",
        "E-TYPE" => "A field's value does not have the field's type. Lists are checked element by element.\n\
            Fix: write the value in the expected form.",
        "E-UNCOVERED" => "A goal of the brief is answered by no commitment.\n\
            An attribute is an observation, not an answer.\n\
            Fix: add a value or a question that addresses the goal.",
        "E-UNDEFINED" => "A reference names a declaration that does not exist.\n\
            Fix: declare it, or correct the spelling.",
        "E-UNFALSIFIABLE" => "A hypothesis has neither a test nor a rival, so no observation could refute it.\n\
            Fix: add a test, name a rival, or record a verdict with the fact that decided it.",
        "E-UNGROUNDED" => "A concept has no positive example, so it denotes nothing.\n\
            Fix: ground it in at least one fact.",
        "E-VACUOUS" => "A proposition names no concept and cites no premise, so it is anchored in nothing.\n\
            Fix: name the concepts it talks about, or the premises it rests on.",
        "E-WARRANT-MODE" => "A warrant must conclude exactly one of a named proposition or a class of them.\n\
            Fix: use `to:` for one conclusion, or `concludes:` for every match.",
        "E-WITNESS" => "A blind spot needs exactly two witnesses.\n\
            Fix: name the pair that the current signature cannot separate.",
        "W-BY-MISMATCH" => "The declared inference type disagrees with the structure: a warrant makes deduction, concepts without a rule make induction.\n\
            Fix: change `by`, or add the rule the declaration claims.",
        "W-COARSE" => "A question has fewer answers than the hypotheses it separates.\n\
            Fix: give each hypothesis an answer.",
        "W-CONDITIONAL" => "A conclusion is supported only through a premise that never reaches a fact.\n\
            Fix: discharge the premise, or state the conclusion as conditional.",
        "W-CONFIDENCE-ASSERTED" => "A proposition carries a confidence but no rule computes it.\n\
            Fix: add the rule, or treat the number as the author's own estimate.",
        "W-CONFIDENCE-INFLATED" => "A declared confidence is larger than the weakest link behind it: a rule's premises, or the premises it cites.\n\
            Fix: lower the number, or strengthen what it rests on.",
        "W-CONFIDENCE-MODEL" => "A model-supplied fact claims high confidence.\n\
            Fix: confirm it, or lower the number.",
        "W-EXAMPLE-SHAPE" => "An example shares no recorded attribute with the concept's positives, so it cannot be compared with them.\n\
            Fix: record an attribute the examples share, or move it out of the list.",
        "W-EXPOSE-FOREIGN" => "A layer exposes a name that sits in another layer.\n\
            Fix: expose it from its own layer.",
        "W-HOLE" => "A value is `?`: a known unknown. It compiles, and it becomes a question.\n\
            Fix: fill it in, or leave it as a deliberate hole.",
        "W-LAW-UNCHECKED" => "A law is asserted, but nothing would break if it were false.\n\
            Fix: point checked_by at the thing that would fail.",
        "W-NO-EXIT" => "A blind spot closes no question, so nothing will ever resolve it.\n\
            Fix: add closes_with pointing at a question.",
        "W-OPEN" => "A proposition is marked open and cites no premises, so nothing could close it.\n\
            Fix: name what would settle it.",
        "W-OVERCLAIM" => "A claim is universal and names no scope.\n\
            Fix: say who or what was sampled, with `scope:`.",
        "W-PARTITION-CLOSED-UNJUSTIFIED" => "A list claims to be closed without a reason.\n\
            Fix: add basis naming why nothing else qualifies.",
        "W-PARTITION-OPEN-USED" => "An open list is used as a premise, so the conclusion rests on a list known to be incomplete.\n\
            Fix: close it with a basis, or stop using it as support.",
        "W-PARTITION-UNKNOWN" => "A pile has no dimension yet: it may be written and may not carry anything.\n\
            Fix: answer the question it names, or add a dimension.",
        "W-PARTITION-UNLEARNED" => "A partition leans on a dimension whose own examples no recorded attribute separates.\n\
            Fix: record the attribute that distinguishes them.",
        "W-UNCONFIRMED" => "A fact was supplied by a model and nobody confirmed it.\n\
            Fix: add confirmed_by, or change the evidence to a person or a tool.",
        "W-UNTESTED" => "A concept is never contrasted with a counterexample.\n\
            Fix: add `against:` with a fact the concept should reject.",
        "W-UNWARRANTED" => "A proposition cites premises, but no rule connects them to it.\n\
            Fix: add the warrant, or drop the premises.",
        "W-UNWARRANTED-BASIS" => "A rule states an inference, but nothing makes the rule acceptable.\n\
            Fix: add basis naming what licenses it.",
        _ => return None,
    })
}

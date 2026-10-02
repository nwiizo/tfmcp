# Coupling Signal Integrity

Use coupling and similarity reports to investigate maintenance cost, not as
scores to improve independently of behavior.

- Keep the analysis scope, Git history window, thresholds, exclusions, and
  subdomain classification comparable before and after a change. Do not disable
  Git, relocate files to hide churn, or alter configuration to suppress findings.
- Confirm findings in source and callers before refactoring. Expected entrypoint
  fan-out, crate-root re-exports, and stable shared domain types are not defects
  merely because they have many dependencies or consumers.
- Consider strength, distance, and business-driven volatility together. Strong
  dependencies across distant, frequently changing responsibilities deserve
  investigation; stable shared knowledge can be appropriate. Temporal co-change
  alone does not establish shared business volatility or a runtime dependency.
- Report missing history, configuration uncertainty, and the analysis manifest's
  unanalysed areas. Do not call a report clean solely because its grade is high.
- Preserve the gates in [quality-commands.md](quality-commands.md) and
  `Release.sh`. Improve structure while preserving behavior; investigate a
  suspected tool false positive separately rather than bypassing the gate.

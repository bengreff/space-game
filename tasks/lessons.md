# Lessons Learned

## Spec-Code Coupling (2026-02-19)

**Mistake**: Implemented multiple features and bug fixes across a session, then tried to batch-update all spec files at the end. This resulted in code changes being committed without corresponding spec updates, and some changes being missed entirely.

**Rule**: Update the spec file as the LAST step of each individual task, before moving to the next task. The task is: implement code -> update spec -> done. Never defer spec updates.

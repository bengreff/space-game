---
name: extract-specs
description: Initialize a spec based on the current contents of the code base.
license: MIT
metadata:
  author: opnly
  version: "1.0"
  generatedBy: "1.0"
---


Initalize a openspec main specs from the current code base.

This is an **agent-driven** operation - you will read current specs, user input then analyize the code base to find requirements not captured by the current files/create new specs

**Input**: 

Extract the following from the user's request:
- **app** - The app spec
- **domain** — the top level domain for all the capabilities to intialize
- **capabilities** — potential list of capabilities to extract requirements for
- **hints** - user may provide context on what exactly they are looking to be added to the spec,  if not provided, find all capabilities in the domain

app, domain and capability will be lower case and snake case.

 `openspec/specs/<app>/<domain>/<capability>/spec.md`

The domain spec contains requirements and scenarios that apply to all capabilities in the domain.

 `openspec/specs/<app>/<domain>/spec.md`

The app spec contains requirements and scenarios that apply to all capabilities in the domain.

 `openspec/specs/<app>/spec.md`

If any required info is missing, ask the user before proceeding.


The specs may or may not exist.  Assume all requirements are original requirements during this initalization
**Steps**
   a. **Read the main app spec** at `openspec/specs/<app>/spec.md` (may not exist yet)

   b. **Read the main domain spec** at `openspec/specs/<app>/<domain>/spec.md` (may not exist yet)

   b. **Read the main capability spec** at `openspec/specs/<app>/<domain>/<capability>/spec.md` (may not exist yet)

   c. **Find requirements intelligentlly**:

     Only look for requirements specific to the capability.

     If anything is general and more specific to the domain, put it in the domain.spec

     If anything would apply to anything running in the app, put the requirement in the app spec

   d. Show potential changes to the app/spec.md
      - Ask user if they want to
        1. Make the changes
        2. Ignore the changes
        3. Apply the requirements to the domain/spec.md instead
   e. Show potential changes to the app/<domain>/spec.md
      - Ask user if they want to
        1. Make the changes
        2. Ignore the changes
        3. Apply the requirements to every capability instead instead

   d. **Create new app spec** if capability doesn't exist yet:
      - Create `openspec/specs/<app>/<domain>/<capability>/spec.md`
      - Identify key folders for the app
      - Add Purpose section (can be brief)
      - Add Requirements section impled requirements from the current code

   d. **Create new main spec** if capability doesn't exist yet:
      - Create `openspec/specs/<app>/<domain>/<capability>/spec.md`
      - Add Purpose section (can be brief)
      - create reference to app spec at the top of the domain spec
      - Add Requirements section impled requirements from the current code

   d. **Create new domain spec** if capability doesn't exist yet:
      - Create `openspec/specs/<app>/<domain>/<capability>/spec.md`
      - Add reference to the domain spec at the top of the capability spec
      - Add Purpose section (can be brief)
      - Add Requirements section impled requirements from the current code

1. **Show summary**

   After applying all changes, summarize:
   - Which capabilities were discovered
   - What changes were made (requirements added)

**Key Principle: Intelligent Merging**

**Output On Success**

```
## Specs Initialized: <app> <domain> <capability>

Modified app spec

- Requirement: X

Created domain spec:

**common requirements**
- Requirement: Requirement: "New Feature" (2 scennarios)

- Requirement: "New Feature" (2 scennarios)

**<capability-1>**:
- Requirement: "New Feature" (2 scennarios)

**<capability-2>**:
- Created new spec file
- Added requirement: "Another Feature"



```

**Guardrails**
- Read both app and domain specs before making changes
- If something is unclear, ask for clarification
- Show what you're changing as you go

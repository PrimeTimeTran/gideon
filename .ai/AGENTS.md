# GIDEON Context

This file defines the global behavior and constraints for the AI runtime across the entire workspace.

It is always loaded at startup and serves as the highest-level source of truth for how context is selected, how tools are used, and how reasoning boundaries are enforced.

All downstream package-level or module-level AI configurations must conform to these rules and may refine behavior only within their scoped domain, never override global constraints.

---

## 🧠 Layer A — Global intent (always loaded)

```
./ai/agents.md
```

This layer defines:

- global system behavior and reasoning rules
- context routing and selection policies
- tool access permissions and restrictions
- workspace-wide safety and execution constraints

This is the **operating system configuration** for the AI runtime and applies to all projects, packages, and modules.

---

## 📦 Layer B — Package / module context (lazy loaded)

```
./<workspace-unit>/.ai/agents.md
```

Where `<workspace-unit>` can be any logical boundary such as:

- package (pnpm, npm, yarn workspaces)
- crate (Cargo)
- service (backend monoliths)
- app (frontend apps)
- module (any directory-level boundary)

This layer defines:

- domain-specific behavior and intent
- architecture and design constraints for that unit
- local reasoning rules and conventions
- tool preferences or restrictions specific to that unit

BUT:

> It is NOT loaded by default.

It is only loaded when the runtime determines that the task is relevant to that unit.

---

## 🔍 Layer C — Source / implementation context (on-demand)

```
./<workspace-unit>/**
```

This includes:

- source code
- configuration files
- tests
- documentation inside the unit

Only loaded when required to complete a task with sufficient accuracy.

---

# 2. Core operating principle

> Load the minimum context required to complete the task correctly, then expand only when additional detail is necessary.

This ensures:

- minimal noise
- high relevance
- scalable performance in large workspaces
- stable reasoning across unrelated modules

---

# 3. Context model: task-driven, not filesystem-driven

The system does NOT operate by scanning directories.

Instead, it operates by interpreting tasks and activating only relevant workspace units.

Meaning:

- workspace structure does NOT imply loading
- presence of a module does NOT imply relevance
- context is activated only through task necessity

---

# 4. Runtime execution flow

## Step 1 — Load global context

Always load:

```
./ai/agents.md
```

This establishes global rules and constraints.

---

## Step 2 — Interpret task intent

The runtime determines:

- which workspace unit(s) are relevant
- what domain(s) the task belongs to
- whether multiple units are involved

Example:

> "refactor VFS logic"

→ activates:

```
vfs module
```

---

## Step 3 — Load module-level context (selective)

Only load:

```
./<workspace-unit>/.ai/agents.md
```

for the activated unit(s).

No other modules are loaded unless explicitly required.

---

## Step 4 — Expand implementation context (on-demand)

If additional detail is required for correctness:

```
./<workspace-unit>/**
```

Expansion must be incremental and justified by task needs.

---

# 5. Context loading rules

## ❌ Never:

- load all `.ai/` directories automatically
- scan entire workspace preemptively
- load unrelated modules “just in case”
- merge all module contexts by default

## ❌ Avoid:

- cross-module contamination
- global context explosion
- unnecessary tool invocation

## ✅ Always:

- start minimal
- expand only when needed
- keep module boundaries isolated unless explicitly crossed

---

# 6. Workspace unit activation model

Each workspace unit is treated as an independent reasoning boundary:

```
WorkspaceUnit {
    path: string
    ai_context: optional
    dependencies: optional other units
}
```

A unit becomes active only when:

- task explicitly references it
- file paths fall within its boundary
- dependency relationships require it
- runtime confidence requires expansion

---

# 7. Context router responsibility

The runtime MUST include a context routing layer responsible for:

### 1. Relevance detection

Determining which workspace units are relevant to the task.

### 2. Scope control

Ensuring only necessary units are active.

### 3. Depth control

Deciding whether to load:

- only AI context
- AI + partial source
- full unit context

### 4. Isolation enforcement

Preventing unrelated units from influencing reasoning.

---

# 8. Global execution principle

> Relevance over completeness.

The system prioritizes:

- precision of context
- correctness of scope
- minimal necessary loading

over:

- full workspace visibility
- exhaustive preloading
- global reasoning across unrelated domains

---

# 9. Summary architecture

```

GLOBAL LAYER
./ai/agents.md

TASK LAYER
determines relevant workspace units

UNIT LAYER
./<workspace-unit>/.ai/agents.md

SOURCE LAYER
./<workspace-unit>/**

```

---

# 10. Core invariant

> The AI runtime must never assume relevance from existence alone. All context must be explicitly or implicitly justified by the task.

```

```

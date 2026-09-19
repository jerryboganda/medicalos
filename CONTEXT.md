# Medical OS Domain Language

Medical OS is a learning platform with one human identity that can operate in a personal learning context and in explicitly authorized institution contexts.

## Language

**Personal context**:
The learner-owned context that exists independently of any institution membership.
_Avoid_: Personal tenant, default institution

**Tenant**:
An institution-level private authorization namespace whose records and privileged operations must remain isolated from other tenants.
_Avoid_: Account, workspace, customer database

**Tenant membership**:
The relationship that authorizes one user to enter one tenant context; one user may hold memberships in multiple tenants.
_Avoid_: Tenant user, institution account

**Tenant role**:
A role held inside one tenant membership, such as learner, instructor, medical reviewer, or institution administrator; a membership may hold multiple tenant roles.
_Avoid_: Global role, account type

**Platform role**:
A cross-tenant operational role, currently support or platform owner, that is separate from every tenant membership.
_Avoid_: Tenant role, super-admin membership

**Audit event**:
An append-only record of a privileged action, including actor, tenant scope, subject, aggregate, timestamps, correlation identifier, privacy scope, and payload.
_Avoid_: Activity log, mutable history

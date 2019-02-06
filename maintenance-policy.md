
# Maintenance Policy

The Maintenance Policy defines how we make decisions about what happens with
Habitat Builder and associated software projects. It provides the process by which:

* Patches are merged
* Disputes are resolved

It is intended to be short, flexible, and clear.

This file is related to the [MAINTAINERS
file](https://github.com/habitat-sh/builder/blob/master/MAINTAINERS.md).

# How the project is maintained

This file is the canonical source for how the Builder project is maintained.

# Roles

## Project Lead

* Resolves disputes
* Provides vision and roadmap
* Has universal veto power
* There can be only one

## Project Sub-Lead

* Each component in the project may have at most one Sub-Lead
* Provides guidance on future direction for their component
* Resolves disputes within their component
* Has localized veto power
* Plus all the responsibilities of a Maintainer

## Maintainer

* Each component may have multiple Maintainers
* Handles contributions on GitHub and reviews/responds to PRs in a reasonable amount of time
* Is available on the Habitat Forum to answer questions
* Is available for [On-Call Responsibilities](https://forums.habitat.sh/t/on-call-engineering-duties/626)
* Committed to 100% tests passing for the component
* Has full commit/merge access to the relevant repositories

# Contributing Patches

## How a patch gets merged

* Open a Developer Certificate of Origin-signed (DCO-signed) Pull Request
  (anyone)
* Code reviewed by a Maintainer, Sub-Lead, or Project Lead. Approval is
  indicated by Github approval on the pull request.
* Merged after approval by at least one Maintainer for the component(s)
  affected by your patch.

### Pull Request Review

The Github Code Review workflow is enabled for the Builder repo.

Approval from at least one Maintainer is required before a PR may be merged.

Any Maintainer may request changes to a patch. The Change Request is required to
be satisfied prior to merge of the PR, unless it is redacted by the Maintainer,
or approved by an absolute majority of Maintainers for the affected
component(s).

## Patch Appeals Process

There may be cases where someone wishes to appeal a Maintainer decision. In this
event, the "chain of command" for the appeals process is as follows.

* In the event that the actions of a Maintainer are to be appealed, the appeal
  should be directed to the Sub-Lead for that component. As stated above, a
  Sub-Lead retains veto power for the component(s) for which they are
  responsible.

* In the event that the actions of a Sub-Lead are to be appealed, the appeal
  should be directed to the Project Lead. As stated above, the Project Lead
  retains universal veto power over all components.

Although Sub-Leads and the Project Lead retain veto powers over certain
components, use of this veto power is not guaranteed by the submission of an
appeal to that person. It is expected that the majority decisions of component
Maintainers and Sub-Leads will be respected in all but the most exceptional
circumstances.

# How to become a...

## Maintainer

* Have patches merged into the relevant component
* Be willing to perform the duties of a Maintainer
* Issue a pull request adding yourself to the MAINTAINERS file for your
  component
* Receive an absolute majority of existing Maintainers and Sub-Leads for your
  component via Approvals of the pull request
* No veto from the component Sub-Lead
* No veto from the current Project Lead

## Sub-Lead

* Issue a pull request to the MAINTAINERS file making yourself the Sub-Lead
* Be willing to perform the duties of a Sub-Lead
* Receive an absolute majority of existing Sub-Leads via Approvals on the pull
  request
* No veto from the current Project Lead

## Project Lead

* Issue a pull request to the MAINTAINERS file making yourself the Project Lead
* Be willing to perform the duties of the Project Lead
* Receive an absolute majority of existing Sub-Leads via Approval on the pull
  request
* No veto from Chef Software, Inc., as held by their current Chief Executive
  Officer.

# Removing a Maintainer, Sub-Lead or Project Lead

If a Maintainer, Sub-Lead, or Project Lead consistently fails to maintain
their responsibilities or becomes disruptive, they can be removed by:

* Issue a pull request removing them from the MAINTAINERS file
* Receive an absolute majority of existing Sub-Leads via Approval on the pull
  request
* No veto from the current Project Lead

OR

* Issue a pull request removing them from the MAINTAINERS file
* The current Project Lead unilaterally decides to merge pull request

# How to add a component

* Issue a pull request to the MAINTAINERS file describing the component, and
  making yourself Sub-Lead
* Be willing to perform the duties of a Sub-Lead
* Receive an absolute majority of existing Sub-Leads via Approval on the pull
  request
* No veto from the current Project Lead

# How to change the rules by which the project is maintained

* Issue a pull request to this file.
* Receive an absolute majority of existing Sub-Leads from the Habitat
  repository MAINTAINERS file via Approval on the pull request
* No veto from the current Project Lead

# The MAINTAINERS file in Builder

The current
[MAINTAINERS](https://github.com/habitat-sh/builder/blob/master/MAINTAINERS.md)
file resides in the [builder](https://github.com/habitat-sh/builder/) repository
on GitHub.

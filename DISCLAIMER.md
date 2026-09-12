# Disclaimer, Assumption of Risk, and Limitation of Liability

**READ THIS BEFORE USING, BUILDING, DISTRIBUTING, OR CONTRIBUTING TO OMNISEARCH.**

omnisearch is an independent open-source project provided solely for convenience. It is **not** a product, service, professional engagement, escrow, audit, insurance policy, marketplace listing, or warranty of any kind.

By cloning, downloading, building, running, configuring, embedding, redistributing, or otherwise using this software (or any fork, binary, container, or derivative), **you accept this document in full**, together with the [Apache License 2.0](LICENSE), [NOTICE](NOTICE), [CONTRIBUTING.md](CONTRIBUTING.md) (if you contribute), and [SECURITY.md](SECURITY.md).

If you do not agree, **do not use the software**. Uninstall it, delete copies, and do not open issues demanding fixes, refunds, or damages.

---

## 0. Free / gratuitous software — no paid bargain with maintainers

- This software is offered under a **free** open-source license. You typically pay **$0** to the copyright holder and other Project Parties for the software itself.
- Publication of source code on GitHub (or elsewhere) is a **gratuitous courtesy**, not a sale, not a consumer transaction, not a SaaS subscription, and not an invitation to treat Project Parties as a vendor of paid goods or services.
- Opening an issue, starring the repo, or receiving a merge **does not** create a support contract, paid engagement, or duty of care.
- **You may not claim** that Project Parties “sold,” “supplied as a product,” or “guaranteed” this code to you in exchange for money paid to them for the software (again: typically **$0**).
- If you need warranties, SLAs, indemnities from a vendor, insurance, or certified secure software — **buy that from a commercial vendor**. Do not use omnisearch for that purpose.

---

## 1. No affiliation

omnisearch is **not** affiliated with, endorsed by, sponsored by, or partnered with any third-party search, social, extract, cloud, AI, payment, or API vendor. Names appear only to identify interoperable endpoints. All trademarks remain with their owners.

---

## 2. Absolute “AS IS” — no warranties of any kind

TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW:

- THE SOFTWARE, DOCUMENTATION, RELEASES, BINARIES, CI ARTIFACTS, DEPENDENCIES, AND ALL RELATED MATERIALS ARE PROVIDED **“AS IS” AND “AS AVAILABLE”**, WITH **ALL FAULTS**.
- THERE ARE **NO** WARRANTIES OR CONDITIONS, EXPRESS, IMPLIED, STATUTORY, OR OTHERWISE — INCLUDING MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, TITLE, NON-INFRINGEMENT, ACCURACY, QUIET ENJOYMENT, OR ARISING FROM COURSE OF DEALING OR USAGE OF TRADE.
- THE COPYRIGHT HOLDERS, AUTHORS, MAINTAINERS, COMMITTERS, AND CONTRIBUTORS (collectively, **“Project Parties”**) **DO NOT WARRANT** that the software is secure, free of defects, free of malware, free of backdoors, correctly reviewed, correctly merged, correctly released, or suitable for any use.
- Project Parties have **no duty** to review, audit, pen-test, monitor supply chains, cap your API spend, or refuse any contribution. Any review that occurs is **voluntary, incomplete, and non-reliance**.

Search and extract output may be wrong, incomplete, biased, stale, or fabricated by upstream providers. It is **not** legal, medical, financial, or other advice.

This section supplements — and does not narrow — Section 7 (Disclaimer of Warranty) and Section 8 (Limitation of Liability) of the [Apache License 2.0](LICENSE).

---

## 3. Assumption of risk (including supply chain)

**YOU ASSUME ALL RISK** of use. Without limitation, you alone assume risk of:

### 3.1 Supply-chain and build-pipeline compromise

- Malicious or compromised crates, git dependencies, registries (including crates.io), mirrors, or transitive dependencies
- Typosquatting, dependency confusion, compromised maintainer accounts upstream of this project
- Compromised GitHub Actions, runners, caches, release workflows, signed or unsigned artifacts, or tag/commit substitution
- Compromised developer machines, forks, mirrors, CDNs, or package mirrors you use to obtain the code
- Backdoors, crypto-miners, credential stealers, or subtle logic bombs introduced in any dependency **or in this repository**

**You must independently verify** checksums, review source, pin and vendor dependencies as you see fit, build from known commits, and apply your own security controls. Project Parties do **not** guarantee the integrity of any download path.

### 3.2 Malicious or defective contributions (including accidental merge)

- Pull requests, issues, docs, workflows, or review comments may contain **intentional or unintentional** harmful code or instructions
- Maintainers **may merge bad, malicious, negligent, or incomplete changes by mistake**, under time pressure, via compromised accounts, or without adequate review
- Green CI **does not** mean safe, audited, or non-malicious
- **You must treat every release and every commit as untrusted until you personally (or your security process) has reviewed it**

Project Parties have **zero responsibility** for harm from merged contributor code, whether or not a maintainer “should have caught it.”

**You expressly waive** claims against Project Parties based on: failure to review a PR; inadequate review; trusting CI; trusting a contributor; merging too quickly; or “negligent maintainership” related to contribution handling — to the maximum extent permitted by law (including where such claims would otherwise be framed as negligence or gross negligence, except only where mandatory law makes that waiver void).

### 3.3 API keys, credits, quotas, and overusage

You supply all API keys, tokens, and accounts. **You alone** are responsible for:

- **All** usage charges, overages, prepaid credit burn, unexpected invoices, and account bans
- Spend caused by bugs, infinite loops, misconfiguration, parallel fan-out, research/extract, agent autonomy, compromised clients, or **malicious code in this project or its dependencies**
- Rate-limit exhaustion and vendor penalties
- Compliance with each vendor’s terms of service and acceptable use policies

Budget flags and env vars (**if any**) are **best-effort hints only**. They can fail, be bypassed, be mis-set, or be removed by a future change. **There is no spend guarantee.**

**If you cannot afford unbounded API bills, do not configure paid providers. Do not give agents unsupervised access to keys.**

### 3.4 Operational and data risks

- Data loss, downtime, SSRF residual risk, credential leakage from your config, privacy breaches at third parties, regulatory violations from your queries
- Exposure of HTTP mode to networks you did not lock down
- Any reliance on health tools, logs, or docs that omit or sanitize fields

---

## 4. Your keys, your compliance

You are solely responsible for lawful use, consents, secrecy of credentials, and regional restrictions. Project Parties do not operate third-party APIs and are not parties to your contracts with those vendors.

---

## 5. No liability — maximum carve-out

TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW:

- **NO Project Party SHALL BE LIABLE** for any claim, damage, loss, cost, or expense — whether in contract, tort (including negligence), strict liability, statute, or otherwise — arising out of or related to the software, contributions, releases, docs, or this disclaimer.
- This includes, without limitation: **supply-chain attacks**; **malware in merged PRs**; **API / credit / invoice overages of any size**; lost profits; lost data; business interruption; reputational harm; third-party claims; regulatory fines; and cost of substitute goods or services.
- **EVEN IF** a Project Party was advised of the possibility of such damages, **EVEN IF** a limited remedy fails its essential purpose, and **EVEN IF** the loss was caused by maintainer mistake, incomplete review, or acceptance of a malicious contribution.
- **AGGREGATE LIABILITY OF ALL PROJECT PARTIES, IF ANY LIABILITY IS FOUND NONETHELESS NON-WAIVABLE UNDER MANDATORY LAW, IS LIMITED TO THE LESSER OF (A) AMOUNTS YOU PAID DIRECTLY TO THE COPYRIGHT HOLDER FOR THE SOFTWARE (TYPICALLY $0) OR (B) USD $0 (ZERO).** You acknowledge this software is free and that zero price is part of the bargain.

Some jurisdictions disallow certain exclusions. **Only** those non-waivable rights survive; all else remains disclaimed. If any clause is held unenforceable, the remainder stays in force and shall be reformed to the **maximum** protective effect permitted.

---

## 6. Indemnification (users and redistributors)

To the maximum extent permitted by law, **you agree to defend, indemnify, and hold harmless** all Project Parties from and against any claims, damages, losses, liabilities, costs, and expenses (including reasonable attorneys’ fees) arising out of or related to:

- Your use, deployment, or redistribution of the software
- Your API keys, agents, automations, or spend
- Your violation of third-party terms or law
- Your modification of the software
- Claims by your customers, employers, or end users

---

## 7. Contributors — extra warranties and indemnity

If you submit a contribution (PR, patch, issue with actionable code, workflow, or similar), you additionally agree that:

1. You have the legal right to submit it under Apache-2.0.
2. To your knowledge it does **not** intentionally include malware, backdoors, crypto-miners, credential exfiltration, hidden spend amplifiers, or license-poisoning material.
3. You will **not** knowingly submit code designed to harm users, Project Parties, or third-party vendors.
4. You **indemnify** Project Parties for claims arising from your intentional malicious contribution or your willful misrepresentation of authorship/rights.
5. Merge or discussion of your contribution **creates no employment, partnership, joint venture, or duty of care** toward you, and **no warranty** that maintainers will detect defects.

Submitting a contribution is **acceptance** of [CONTRIBUTING.md](CONTRIBUTING.md) and this section.

---

## 8. No reliance; no professional relationship

You must not rely on Project Parties’ silence, merges, stars, CI status, or docs as a security assurance. Nothing here creates a fiduciary, advisory, employment, joint venture, or professional-services relationship.

**No third-party beneficiaries** except that every Project Party may enforce this disclaimer and the Apache-2.0 limitations. Your customers, employers, and end users have **no** direct claim against Project Parties arising from your use of omnisearch; you alone stand between them and this software (see §6).

---

## 8A. Claims you agree not to bring

To the maximum extent permitted by law, you agree **not** to bring (and you waive) claims against Project Parties for:

- Free software “should have been safer / reviewed better”
- Damages after you ran code you did not personally review
- API bills, credit burn, or vendor bans of any amount
- Supply-chain compromise of dependencies or CI
- Harm from a contribution a maintainer merged
- Lack of support, delayed security response, or ignored issues
- Reliance on README examples, defaults, or sample configs

Bring disputes only as allowed by mandatory law that cannot be waived — and even then, subject to the **$0** aggregate cap in §5 where enforceable.

---

## 9. Security and privacy (operational)

- Queries and URLs are sent to providers **you** configure; treat them as shared with those third parties.
- Do not place secrets or regulated data in queries without lawful basis and vendor permission.
- Prefer stdio MCP; if you enable HTTP, authenticate strongly and do not expose it publicly without your own perimeter.
- Built-in fetch/SSRF guards are incomplete mitigations, not guarantees.

---

## 10. Reporting

- Vulnerabilities: [SECURITY.md](SECURITY.md) (private). Reporting does **not** create a service-level obligation or liability for delayed or incomplete response.
- Other help: [SUPPORT.md](SUPPORT.md). Support is best-effort and may be none.

---

## 11. Precedence

If this document conflicts with informal statements (chat, issues, social media), **this document and the Apache-2.0 license control**. Stronger protective language in this file is intended to **expand** disclaimers for users of this repository and does not reduce Apache-2.0 permissions granted to licensees who comply with that license.

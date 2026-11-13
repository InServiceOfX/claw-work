# Harsh Gupta (hargup) repos: survey for Lean-verified optimization

Surveyed 2026-11-01 by Grok Bot (yeung-assistant). Total public repos (including forks): 78. Context: his `lean-derive-hargup` submission to arjaythedev/twelve-dollar-server-challenge specifies an API in Lean 4, proves optimized query plans equal the reference under a data invariant, and generates C.

Each repo was judged from its name, description, and languages; candidates and repos with empty or vague descriptions were also checked via README or file tree. Code is **not** vendored here. Reference clones live on the Grok Bot box only, under `/workspace/references/hargup/<repo>`.

## Relevant repos

### twelve-dollar-server-challenge (fork)
Local path: `/workspace/references/hargup/twelve-dollar-server-challenge` (full clone, about 15 MB; branches `lean-derive-hargup`, `lean-derive-lowmem-hargup`, `lean-derive-metal-complete-hargup` tracked locally). This is the fork for the $12-server SQLite API benchmark. His submission (`submissions/lean-derive-hargup`, also merged upstream as PR #153 "Lean 4 derived C-Code") writes the API spec in Lean 4. It proves that the optimized query plans are equal to the reference semantics under a data invariant, then emits C. That makes it a direct example of Lean-verified optimization with code generation. The lowmem and metal branches are further performance variants.

### theoric
Local path: `/workspace/references/hargup/theoric` (full clone). This is the source of theoric.com, the site for his applied AI lab at the intersection of formal methods and agentic AI. Its blog posts (Markdown sources under `blog/`) cover LeanDB (a strongly typed SQL frontend), LeanAPI, LeanHTTP, LeanReact, domain-driven development in Lean, "We rewrote tinygrad in Lean", "Why Lean 4 for agentic coding", and "The future of software will be bug-free". These posts lay out the design thinking behind the spec, prove, and generate approach.

## Borderline repos (cloned as small Lean 4 code examples)
- `cli-chess`: `/workspace/references/hargup/cli-chess`
- `lean-http-client`: `/workspace/references/hargup/lean-http-client`
- `lean-ai-sdk`: `/workspace/references/hargup/lean-ai-sdk`

## All repos

| Repo | Fork | Language | Relevance | Reason | URL |
|---|---|---|---|---|---|
| twelve-dollar-server-challenge | yes | C | relevant | Fork holding his lean-derive-hargup submission (Lean 4 spec, proved-equal optimized query plans, C codegen) plus lowmem/metal variant branches | https://github.com/hargup/twelve-dollar-server-challenge |
| theoric | no | HTML | relevant | Site of Theoric, his formal methods + agentic AI lab; posts on LeanDB, LeanAPI, rewriting tinygrad in Lean, bug-free software | https://github.com/hargup/theoric |
| cli-chess | no | Lean | borderline | Chess engine written in Lean 4; Lean programming, but no proofs or optimization focus | https://github.com/hargup/cli-chess |
| lean-http-client | no | Lean | borderline | Type-safe Lean 4 HTTP client over curl FFI; Lean systems programming, not verification | https://github.com/hargup/lean-http-client |
| locality-upstream | yes | Rust | not relevant | Unrelated to Lean/formal methods (Locality: agent-local file system access to systems of record for safe local editing and A) | https://github.com/hargup/locality-upstream |
| hargup.github.io | no | HTML | not relevant | Unrelated to Lean/formal methods (My personal website at ) | https://github.com/hargup/hargup.github.io |
| pi-mono | yes | TypeScript | not relevant | Unrelated to Lean/formal methods (AI agent toolkit: coding agent CLI, unified LLM API, TUI & web UI libraries, Slack bot, vL) | https://github.com/hargup/pi-mono |
| felvin.com | no | HTML | not relevant | Redirect page to clockworks-compute.com | https://github.com/hargup/felvin.com |
| moltbot | yes | TypeScript | not relevant | Unrelated to Lean/formal methods (Your own personal AI assistant. Any OS. Any Platform. The lobster way. 🦞 ) | https://github.com/hargup/moltbot |
| lean-ai-sdk | no | Lean | borderline | LLM SDK in Lean 4 (functional core / imperative shell); Lean programs, not proofs | https://github.com/hargup/lean-ai-sdk |
| stagehand | yes | TypeScript | not relevant | Unrelated to Lean/formal methods (An AI web browsing framework focused on simplicity and extensibility.) | https://github.com/hargup/stagehand |
| reader | no | TypeScript | not relevant | Unrelated to Lean/formal methods (Clone of https://r.jina.ai which is deployable locally) | https://github.com/hargup/reader |
| copy_cookies | no | - | not relevant | Empty repo | https://github.com/hargup/copy_cookies |
| visbrowser | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/visbrowser |
| express-ts-llm-template | no | TypeScript | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/express-ts-llm-template |
| green_hydrogen_financial_model | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/green_hydrogen_financial_model |
| WebAgentEval | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/WebAgentEval |
| twitter_bulk_unfollower | no | Python | not relevant | Unrelated to Lean/formal methods (Unfollower twitter accounts in bulk.) | https://github.com/hargup/twitter_bulk_unfollower |
| misc | no | Python | not relevant | Small Python scratch scripts (no README) | https://github.com/hargup/misc |
| basic-github-bot | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/basic-github-bot |
| gkeepapi | yes | Python | not relevant | Unrelated to Lean/formal methods (An unofficial client for the Google Keep API.) | https://github.com/hargup/gkeepapi |
| GPT-einops | no | Jupyter Notebook | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/GPT-einops |
| instant-apps | yes | JavaScript | not relevant | Unrelated to Lean/formal methods (Your search box is now an app store! 🎉) | https://github.com/hargup/instant-apps |
| ReactSSRExperiment | no | JavaScript | not relevant | Unrelated to Lean/formal methods (Experimenting with Server Side Rendering in React) | https://github.com/hargup/ReactSSRExperiment |
| contractx | no | - | not relevant | Unrelated to Lean/formal methods (Notes from edX course "Contract Law: From Trust to Promise to Contract") | https://github.com/hargup/contractx |
| covidsupport | no | JavaScript | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/covidsupport |
| stackexchange-dump-to-postgres | yes | Python | not relevant | Unrelated to Lean/formal methods (Python scripts to import StackExchange data dump into Postgres DB.) | https://github.com/hargup/stackexchange-dump-to-postgres |
| twitter-search-covid19 | yes | Svelte | not relevant | Unrelated to Lean/formal methods (Twitter Search for COVID19 resources in your city) | https://github.com/hargup/twitter-search-covid19 |
| neera_default_search_extension | yes | - | not relevant | Unrelated to Lean/formal methods (Firefox add-on to add Qwant as the default search engine and open links in same tab withou) | https://github.com/hargup/neera_default_search_extension |
| gsheet_experiment | no | Python | not relevant | Unrelated to Lean/formal methods (Ping a route to increament counter in a google sheet) | https://github.com/hargup/gsheet_experiment |
| server-template-node | no | - | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/server-template-node |
| sample_clojure | no | Clojure | not relevant | Unrelated to Lean/formal methods (A simple CRUD app in Clojure.) | https://github.com/hargup/sample_clojure |
| WIGI-website | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/WIGI-website |
| .emacs.d | no | Emacs Lisp | not relevant | Unrelated to Lean/formal methods (Emacs configuration (adapted from https://github.com/suvratapte/dot-emacs-dot-d)) | https://github.com/hargup/.emacs.d |
| media | no | - | not relevant | List of talks | https://github.com/hargup/media |
| do.it | yes | Clojure | not relevant | Unrelated to Lean/formal methods (Collaborative To-Do list) | https://github.com/hargup/do.it |
| intern_diary_2015 | no | Jupyter Notebook | not relevant | Unrelated to Lean/formal methods (Diary for my internship at Continuum Analytics) | https://github.com/hargup/intern_diary_2015 |
| intern_diary_2016 | no | HTML | not relevant | Unrelated to Lean/formal methods (Diary for my 2016 Summer internship at CIS-India http://cis-india.org/) | https://github.com/hargup/intern_diary_2016 |
| time-tracker-web-nxt | yes | CSS | not relevant | Unrelated to Lean/formal methods (A time tracker application brought to you by ClojureScript with all the goodness of re-fra) | https://github.com/hargup/time-tracker-web-nxt |
| takoyaki | yes | JavaScript | not relevant | Unrelated to Lean/formal methods (Fuzzy clustering interface prototype.) | https://github.com/hargup/takoyaki |
| Dotfiles-other | yes | Vim Script | not relevant | Unrelated to Lean/formal methods (A bunch of configuration files for other programs..) | https://github.com/hargup/Dotfiles-other |
| gazette-of-india | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/gazette-of-india |
| metakgp-wiki | yes | PHP | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/metakgp-wiki |
| dotfiles | no | Vim Script | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/dotfiles |
| 2014-app-developer-survey | no | Jupyter Notebook | not relevant | Unrelated to Lean/formal methods (Code for analysis of CIS's 2014 app developer survey) | https://github.com/hargup/2014-app-developer-survey |
| diff_pricing_cp | no | TeX | not relevant | Unrelated to Lean/formal methods (A compilation of responses on Differential Pricing Consultation Paper by TRAI) | https://github.com/hargup/diff_pricing_cp |
| slack-export-viewer | yes | Python | not relevant | Unrelated to Lean/formal methods (A Slack Export archive viewer that allows you to easily view and share your Slack team's e) | https://github.com/hargup/slack-export-viewer |
| masters_project | no | TeX | not relevant | TeX M.Sc. dissertation (checked README); not formal methods | https://github.com/hargup/masters_project |
| mfqp | yes | HTML | not relevant | Unrelated to Lean/formal methods (Question paper search) | https://github.com/hargup/mfqp |
| fatml_iiit_a | no | Jupyter Notebook | not relevant | Unrelated to Lean/formal methods (Notes from Fairness, Accountability and Transparency in Machine Learning winter school at ) | https://github.com/hargup/fatml_iiit_a |
| clean_the_doc | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/clean_the_doc |
| git_tutorial | no | Jupyter Notebook | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/git_tutorial |
| ga_curve_fitting | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/ga_curve_fitting |
| awesome_tech_intersection_law | no | - | not relevant | Unrelated to Lean/formal methods (List of Organizations working on the intersection of Technology and Law) | https://github.com/hargup/awesome_tech_intersection_law |
| HackU | no | JavaScript | not relevant | Unrelated to Lean/formal methods (Our hack for the Yahoo HackU 2013 IIT Kharagpur) | https://github.com/hargup/HackU |
| laws_of_india | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/laws_of_india |
| resume | yes | TeX | not relevant | Unrelated to Lean/formal methods (Resume) | https://github.com/hargup/resume |
| work_logs | no | GCC Machine Description | not relevant | Personal work logs | https://github.com/hargup/work_logs |
| sympy | yes | Python | not relevant | Old 2014 fork of the SymPy CAS (his GSoC solvers work); symbolic math, not theorem proving | https://github.com/hargup/sympy |
| eme_diversity_analysis | no | Jupyter Notebook | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/eme_diversity_analysis |
| bigbang | yes | Python | not relevant | Unrelated to Lean/formal methods (Scientific analysis of collaborative communities) | https://github.com/hargup/bigbang |
| whgi-indic | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/whgi-indic |
| source_code_ktj | no | Python | not relevant | Unrelated to Lean/formal methods (Repository for the problems of the event "Source Code" at Kshitij 2016) | https://github.com/hargup/source_code_ktj |
| Dotfiles-vim | yes | Vim Script | not relevant | Unrelated to Lean/formal methods (The helpful plugin and setting of VIM !) | https://github.com/hargup/Dotfiles-vim |
| papers_i_read | no | - | not relevant | Unrelated to Lean/formal methods (Summaries of the papers I read) | https://github.com/hargup/papers_i_read |
| python-classes | no | - | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/python-classes |
| judis-re | yes | Python | not relevant | Unrelated to Lean/formal methods (Automatically exported from code.google.com/p/judis-re) | https://github.com/hargup/judis-re |
| pypi-conda-builds | yes | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/pypi-conda-builds |
| posts | no | HTML | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/posts |
| aosd | no | - | not relevant | Unrelated to Lean/formal methods (Amusing OurSelves to Death summary) | https://github.com/hargup/aosd |
| blog | no | CSS | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/blog |
| git-sandbox | yes | - | not relevant | Unrelated to Lean/formal methods (practicing git commands) | https://github.com/hargup/git-sandbox |
| WIGI | yes | HTML | not relevant | Unrelated to Lean/formal methods (Wikipedia Gender Index (WIGI), uses Wikidata to produce gender-related statistic on Wikipe) | https://github.com/hargup/WIGI |
| afra | yes | JavaScript | not relevant | Unrelated to Lean/formal methods (Genome Annotation for the Masses) | https://github.com/hargup/afra |
| gene-simulation | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/gene-simulation |
| bioinformatics | no | Python | not relevant | Unrelated to Lean/formal methods (no description) | https://github.com/hargup/bioinformatics |
| sicp | no | Racket | not relevant | SICP exercise solutions in Racket; functional programming, not verification | https://github.com/hargup/sicp |
| unicode_to_shusha | no | Python | not relevant | Unrelated to Lean/formal methods (Converts hindi text in shusha to ascii shusha font) | https://github.com/hargup/unicode_to_shusha |

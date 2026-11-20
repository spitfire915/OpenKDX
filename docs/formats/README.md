# On-disk formats

The files the original programs keep (`Accounts.dat`, `KDXServer.stg`,
`News.dat`, `Catalog.dat`, `KDXTrackerServ.stg`, the client's `KDX.stg`,
`.hap` themes, the `History` and `News Archives` text) are read and written
by `kdx-legacy` in the kdx fork, and documented there:
https://github.com/spitfire915/kdx/tree/main/docs/formats (upstreamed to
scottanderson/kdx by pull request). The lab fixtures the pages cite live
beside the crate, under `crates/kdx-legacy/tests/fixtures/`; OpenKDX's
`lab/legacy.sh` still produces them and feeds what the crate writes back to
the original programs.

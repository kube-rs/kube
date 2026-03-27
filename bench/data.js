window.BENCHMARK_DATA = {
  "lastUpdate": 1774634373474,
  "repoUrl": "https://github.com/kube-rs/kube",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "doxxx93@gmail.com",
            "name": "doxxx93",
            "username": "doxxx93"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "58cf5a6d762d49e101c304ec060ef60bec2769c3",
          "message": "Add memory benchmark CI workflow (#1937)\n\n* feat(memory): add memory benchmark for kube-runtime watcher/reflector\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(memory): enhance memory benchmarks with structured result collection\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(memory): add GitHub Actions workflow for memory benchmarking\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* style(memory): format output for better readability in memory benchmark results\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(memory): add caching for benchmark data and update comparison step\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(ci): trigger memory benchmark workflow on push to main branch\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(ci): limit memory benchmark workflow to specific paths on push and pull request\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(ci): use gh-pages for baseline, add permissions, apply review feedback\n\n- Switch PR comparison from actions/cache to gh-pages branch\n- Add permissions (contents: write, pull-requests: write) for\n  alert comments and gh-pages auto-push\n- Lower alert threshold to 110%\n- Restrict to github.repository == 'kube-rs/kube'\n\nSigned-off-by: doxxx <doxxx93@gmail.com>\n\n---------\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\nSigned-off-by: doxxx <doxxx93@gmail.com>",
          "timestamp": "2026-02-20T09:42:30+09:00",
          "tree_id": "bf6215ea664faeccebf74c0c63d6612b736fbb94",
          "url": "https://github.com/kube-rs/kube/commit/58cf5a6d762d49e101c304ec060ef60bec2769c3"
        },
        "date": 1771548268859,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "10092581+NickLarsenNZ@users.noreply.github.com",
            "name": "Nick",
            "username": "NickLarsenNZ"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1320643f8ce7f8189e03496ff1329d678d76224c",
          "message": "fix: Produce valid CRDs containing flattened untagged enums (#1942)\n\n* test(kube-derive): Add failing test for a flattened and untagged enum\n\nNote: This comes from #1839, with some modifications (an extra field to the B variant, and change comments to indicate untagged variant descriptions should not leak into fields).\n\nCo-authored-by: Sebastian Bernauer <sebastian.bernauer@stackable.tech>\n\nSigned-off-by: Nick Larsen <nick.larsen@stackable.tech>\n\n* fix(kube-core): Remove lingering type and description fields from variants without fields\n\nSigned-off-by: Nick Larsen <nick.larsen@stackable.tech>\n\n* fix(kube-core/schema): Only push variant descriptions into properties for oneOf\n\nNote: variant descriptions are meaningless for untagged enums - they don't correspond to the fields inside struct variants.\n\nSigned-off-by: Nick Larsen <nick.larsen@stackable.tech>\n\n* chore(kube-core/schema): Adjust comment and move higher\n\nSigned-off-by: Nick Larsen <nick.larsen@stackable.tech>\n\n---------\n\nSigned-off-by: Nick Larsen <nick.larsen@stackable.tech>",
          "timestamp": "2026-02-24T13:51:36Z",
          "tree_id": "9b23e92a22ef2d4d35445d035939d601457892a1",
          "url": "https://github.com/kube-rs/kube/commit/1320643f8ce7f8189e03496ff1329d678d76224c"
        },
        "date": 1771941234322,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "143583906+Immortal-Beyond-Oblivion@users.noreply.github.com",
            "name": "Amar4staz",
            "username": "Immortal-Beyond-Oblivion"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "90975fdb5e0832d7d0baed5e5c2408eb6453e7b1",
          "message": "feat(kube-core): add optionalOldSelf to CEL Rule (#1947)\n\n* feat(kube-core): add optionalOldSelf to CEL Rule\n\nSigned-off-by: Immortal-Beyond-Oblivion <yourlimitisyourknowledge@gmail.com>\n\n* address reviewer feedback: fix CEL optional type and doc nits\n\nSigned-off-by: Immortal-Beyond-Oblivion <yourlimitisyourknowledge@gmail.com>\n\n* doc(cel): clarify optionalOldSelf behavior and update tests\n\nSigned-off-by: Immortal-Beyond-Oblivion <yourlimitisyourknowledge@gmail.com>\n\n---------\n\nSigned-off-by: Immortal-Beyond-Oblivion <yourlimitisyourknowledge@gmail.com>",
          "timestamp": "2026-03-02T15:28:01Z",
          "tree_id": "c777409dca6686118214c81ebcca0fb62be27c8e",
          "url": "https://github.com/kube-rs/kube/commit/90975fdb5e0832d7d0baed5e5c2408eb6453e7b1"
        },
        "date": 1772466455434,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "gauravgahlot0107@gmail.com",
            "name": "Gaurav Gahlot",
            "username": "gauravgahlot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc318bc037c8ffd411589254918594e0e62b3dc2",
          "message": "chore: fix a few typos across the repository (#1949)\n\nSigned-off-by: Gaurav Gahlot <gaurav.gahlot@ionos.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-03-02T16:00:00Z",
          "tree_id": "73725eea1daa70cd38731c19ace499edb07a9190",
          "url": "https://github.com/kube-rs/kube/commit/bc318bc037c8ffd411589254918594e0e62b3dc2"
        },
        "date": 1772469088632,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "doxxx93@gmail.com",
            "name": "doxxx",
            "username": "doxxx93"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ab9040edbfeffb2584d77fa0edfd02eff8f60d2b",
          "message": "fix(runtime): add doc_cfg and remove stale unstable feature notes (#1958)\n\nchore(runtime): remove outdated references to unstable feature flags\n- Cleaned up documentation across multiple modules to remove mentions\n  of deprecated or unnecessary `unstable` feature flags.\n- Added `doc_cfg` feature guard to ensure consistent documentation behavior.\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>",
          "timestamp": "2026-03-10T13:52:10Z",
          "tree_id": "32fdadb320c10f4f5c85e026d6d576408d2bf6ec",
          "url": "https://github.com/kube-rs/kube/commit/ab9040edbfeffb2584d77fa0edfd02eff8f60d2b"
        },
        "date": 1773150928261,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "me@goenning.net",
            "name": "Guilherme Oenning",
            "username": "goenning"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9ad27a9691948d90c27e687cec6bf4a499112ef8",
          "message": "Re-add support for auth in Proxy (#1959)\n\nadd proxy auth\n\nSigned-off-by: goenning <me@goenning.net>",
          "timestamp": "2026-03-15T23:19:07Z",
          "tree_id": "0a77072fa68889857f0fc5e5dd524419ae320b1f",
          "url": "https://github.com/kube-rs/kube/commit/9ad27a9691948d90c27e687cec6bf4a499112ef8"
        },
        "date": 1773616864722,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "5624864+blakelawson@users.noreply.github.com",
            "name": "Blake Lawson",
            "username": "blakelawson"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7424ee37d2cf77026a9ec2ecedbc909278c31601",
          "message": "fix(kube-client): Avoid blocking tokio worker during exec auth token refresh (#1950)\n\n* fix(kube-client): avoid blocking tokio worker during exec auth token refresh\n\nWhen exec-based auth (e.g. aws eks get-token, gcp cmd-path) refreshes its\ntoken, std::process::Command::output() was called synchronously inside an\nasync future's poll, blocking the tokio worker thread for the duration of\nthe exec command (~500ms-2s).\n\nWrap the Auth::try_from call in RefreshableToken::to_header with\ntokio::task::spawn_blocking so the exec command runs on the blocking\nthreadpool instead. The tokio::sync::Mutex guard is held across the await,\ncorrectly serializing concurrent refreshes. Client-construction callers\nremain sync — this only affects the per-request refresh path.\n\nAlso add \"rt\" to tokio features explicitly (already relied on transitively\nvia tokio::spawn usage elsewhere in the crate).\n\nSigned-off-by: Blake Lawson <blake@anthropic.com>\nSigned-off-by: blake <blake@anthropic.com>\n\n* Add tests for async token refresh.\n\nPreviously, there was no test coverage for this code, so this commit\nalso adds a basic correctness test.\n\nSigned-off-by: blake <blake@anthropic.com>\n\n---------\n\nSigned-off-by: Blake Lawson <blake@anthropic.com>\nSigned-off-by: blake <blake@anthropic.com>",
          "timestamp": "2026-03-15T23:41:39Z",
          "tree_id": "88bf607acbd5cc939bc1c0fd30ae9f0f3022600b",
          "url": "https://github.com/kube-rs/kube/commit/7424ee37d2cf77026a9ec2ecedbc909278c31601"
        },
        "date": 1773618152180,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "sszynrae@gmail.com",
            "name": "clux",
            "username": "clux"
          },
          "committer": {
            "email": "sszynrae@gmail.com",
            "name": "clux",
            "username": "clux"
          },
          "distinct": true,
          "id": "a3a111c5b07093aad1a2e229827280f6c47fbd27",
          "message": "release 3.1.0",
          "timestamp": "2026-03-17T11:01:47Z",
          "tree_id": "0038b85a41bf58211a5dbf86248ad6fcbd130640",
          "url": "https://github.com/kube-rs/kube/commit/a3a111c5b07093aad1a2e229827280f6c47fbd27"
        },
        "date": 1773745501264,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "alexeylapuka@gmail.com",
            "name": "Alex Lapuka",
            "username": "alex-lapuka"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0019e54c981f99f7e0a0775be1d7022016f171ae",
          "message": "feat: add typed kubeconfig fields for client-go parity (#1965)\n\nAdd explicitly typed fields to AuthInfo and ExecConfig that are present\nin the upstream client-go v1 kubeconfig types but were previously\nmissing from kube-rs.\n\nNew typed fields:\n- AuthInfo.impersonate_uid (as-uid)\n- AuthInfo.impersonate_user_extra (as-user-extra)\n- AuthInfo.extensions (extensions)\n- ExecConfig.install_hint (installHint)\n\nThese fields are standard in client-go's\ntools/clientcmd/api/v1/types.go and commonly appear in kubeconfig files\ngenerated by cloud providers (e.g. GKE's installHint for\ngke-gcloud-auth-plugin).\n\nSigned-off-by: Alexey Lapuka <alexey@twingate.com>",
          "timestamp": "2026-03-27T11:48:36Z",
          "tree_id": "1e818fd360578fefb8948499375126ad550a49b6",
          "url": "https://github.com/kube-rs/kube/commit/0019e54c981f99f7e0a0775be1d7022016f171ae"
        },
        "date": 1774612231663,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "alexeylapuka@gmail.com",
            "name": "Alex Lapuka",
            "username": "alex-lapuka"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cfa38f21f238e16c7b6b65135c42cf1161d4e510",
          "message": "preserve unknown kubeconfig fields via serde(flatten) (#1964)\n\n* feat: preserve unknown kubeconfig fields via serde(flatten)\n\nAdd `#[serde(flatten)] pub other: BTreeMap<String, serde_json::Value>`\nto all kubeconfig structs (Kubeconfig, Cluster, AuthInfo, ExecConfig,\nContext, Preferences, AuthProviderConfig, and Named* wrappers) so that\nunmodeled fields survive deserialization and can be serialized back\nwithout data loss.\n\nAlso derive Default for ExecConfig, Preferences, and AuthProviderConfig.\n\nUpdate Kubeconfig::merge() to merge extra fields with first-wins-per-key\nsemantics. Add a round-trip test verifying unknown fields are preserved\nacross deserialize/serialize cycles.\n\nSigned-off-by: Alexey Lapuka <alexey@twingate.com>\n\n* Update documentation on all 'other' field catch-alls to suggest that consumers relying on standard client-go fields should submit PRs to add them as typed fields rather than using the generic fallback.\nAlso update the round-trip test to use only non-standard field names to avoid collision when standard fields like installHint are later added as typed fields.\n\nSigned-off-by: Alexey Lapuka <alexey@twingate.com>\n\n---------\n\nSigned-off-by: Alexey Lapuka <alexey@twingate.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-03-27T11:59:22Z",
          "tree_id": "94cc5fe07965b0c4b77ab052ac5cba83fa61231b",
          "url": "https://github.com/kube-rs/kube/commit/cfa38f21f238e16c7b6b65135c42cf1161d4e510"
        },
        "date": 1774612811100,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "doxxx93@gmail.com",
            "name": "doxxx",
            "username": "doxxx93"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8cb0d011d63ab106baa01bc6f95dc679cc645bc5",
          "message": "Remove global read_timeout default, add watcher-level idle timeout (#1945)\n\n* fix: remove default read timeout to support long-lived connections\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* test(watcher): add tests for idle timeout behavior with streams\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* fix(watcher): update timeout parameter naming for clarity\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n---------\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-03-27T17:58:42Z",
          "tree_id": "f2a3e3b48c51f286cb053318dc74849ad3a57aa1",
          "url": "https://github.com/kube-rs/kube/commit/8cb0d011d63ab106baa01bc6f95dc679cc645bc5"
        },
        "date": 1774634372176,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "init_listwatch - peak_bytes",
            "value": 55194619,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - total_allocated",
            "value": 76715088,
            "unit": "bytes"
          },
          {
            "name": "init_listwatch - alloc_count",
            "value": 578023,
            "unit": "allocations"
          },
          {
            "name": "steady_state - peak_bytes",
            "value": 71381202,
            "unit": "bytes"
          },
          {
            "name": "steady_state - total_allocated",
            "value": 109519220,
            "unit": "bytes"
          },
          {
            "name": "steady_state - alloc_count",
            "value": 799021,
            "unit": "allocations"
          },
          {
            "name": "relist - peak_bytes",
            "value": 99797302,
            "unit": "bytes"
          },
          {
            "name": "relist - total_allocated",
            "value": 174518628,
            "unit": "bytes"
          },
          {
            "name": "relist - alloc_count",
            "value": 1189035,
            "unit": "allocations"
          },
          {
            "name": "init_without_modify - peak_bytes",
            "value": 141298836,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - total_allocated",
            "value": 205865000,
            "unit": "bytes"
          },
          {
            "name": "init_without_modify - alloc_count",
            "value": 1298020,
            "unit": "allocations"
          },
          {
            "name": "init_with_modify - peak_bytes",
            "value": 134853452,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - total_allocated",
            "value": 162895000,
            "unit": "bytes"
          },
          {
            "name": "init_with_modify - alloc_count",
            "value": 1058021,
            "unit": "allocations"
          }
        ]
      }
    ]
  }
}
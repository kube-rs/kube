window.BENCHMARK_DATA = {
  "lastUpdate": 1781360725121,
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
      },
      {
        "commit": {
          "author": {
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "561a95e1d3e714176293b74cf1d116325f4f456f",
          "message": "Update tokio-tungstenite requirement from 0.28.0 to 0.29.0 (#1963)\n\nUpdates the requirements on [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) to permit the latest version.\n- [Changelog](https://github.com/snapview/tokio-tungstenite/blob/master/CHANGELOG.md)\n- [Commits](https://github.com/snapview/tokio-tungstenite/compare/v0.28.0...v0.29.0)\n\n---\nupdated-dependencies:\n- dependency-name: tokio-tungstenite\n  dependency-version: 0.29.0\n  dependency-type: direct:production\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-03-27T23:20:24Z",
          "tree_id": "79c113efe572c753229e70f8b559bbbe5204f22a",
          "url": "https://github.com/kube-rs/kube/commit/561a95e1d3e714176293b74cf1d116325f4f456f"
        },
        "date": 1774653678544,
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
            "email": "cristei.g772@gmail.com",
            "name": "gabriela cristei",
            "username": "cristeigabriela"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "796b90d7a5f75d7f104dd2e5cda3eb337efe4ad1",
          "message": "fix: feature-flag CREATE_NO_WINDOW to not break stderr inheritance (#1971)\n\nSigned-off-by: gabriela <gabrielac@metalbear.co>",
          "timestamp": "2026-04-13T16:11:35+01:00",
          "tree_id": "2a9b0810b8458dd7096591a3d4012ae20e64fa97",
          "url": "https://github.com/kube-rs/kube/commit/796b90d7a5f75d7f104dd2e5cda3eb337efe4ad1"
        },
        "date": 1776093210323,
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
            "name": "Eirik A",
            "username": "clux"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e7060a8713aabaa209cb004f49f1ab4d13ff2226",
          "message": "convert from serde-yaml to serde-saphyr (#1975)\n\n* Use serde-saphyr in kube-client\n\nSigned-off-by: John Vandenberg <jayvdb@gmail.com>\n\n* Update deny.toml\n\nSigned-off-by: John Vandenberg <jayvdb@gmail.com>\n\n* Apply suggestion from @clux\n\nSigned-off-by: Eirik A <sszynrae@gmail.com>\n\n* convert stray examples to saphyr\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* minimal versions\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* deny fix\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* fmt + example test\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* box big saphyr error\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* actually don't need the boxed_from\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* source does not add anything here\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n---------\n\nSigned-off-by: John Vandenberg <jayvdb@gmail.com>\nSigned-off-by: Eirik A <sszynrae@gmail.com>\nSigned-off-by: clux <sszynrae@gmail.com>\nCo-authored-by: John Vandenberg <jayvdb@gmail.com>",
          "timestamp": "2026-05-01T16:42:00+01:00",
          "tree_id": "36dd23f6fa0dffa44a53895d15b15bd58cb03705",
          "url": "https://github.com/kube-rs/kube/commit/e7060a8713aabaa209cb004f49f1ab4d13ff2226"
        },
        "date": 1777650249370,
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
          "id": "21b0f61aa9e72cc24e3a90073a78dfb31abccebd",
          "message": "Remove silent error when client-key/client-certificate is malformed (#1966)\n\n* fix silent err\n\nSigned-off-by: goenning <me@goenning.net>\n\n* remove unused\n\nSigned-off-by: goenning <me@goenning.net>\n\n* fix unit tests\n\nSigned-off-by: goenning <me@goenning.net>\n\n* fix openssl compile error\n\nSigned-off-by: goenning <me@goenning.net>\n\n---------\n\nSigned-off-by: goenning <me@goenning.net>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-05-04T10:20:01+01:00",
          "tree_id": "9194d705dc9ac7c910f0bc857e232a4bb0e2a8bb",
          "url": "https://github.com/kube-rs/kube/commit/21b0f61aa9e72cc24e3a90073a78dfb31abccebd"
        },
        "date": 1777886464171,
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
            "email": "mattklein123@gmail.com",
            "name": "Matt Klein",
            "username": "mattklein123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "26a42f2b95d7276ee11e95de0d68a7776748b321",
          "message": "features: making client tracing opt-in (#1972)\n\nDue to https://github.com/tokio-rs/tracing/issues/3521 I would like to\nrequest that this feature be opt-in.\n\nSigned-off-by: Matt Klein <mklein@bitdrift.io>",
          "timestamp": "2026-05-04T10:21:08+01:00",
          "tree_id": "816d808d1cdb7caad6d08f445e385d01a0e82d60",
          "url": "https://github.com/kube-rs/kube/commit/26a42f2b95d7276ee11e95de0d68a7776748b321"
        },
        "date": 1777886519148,
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
            "email": "17420369+chrnorm@users.noreply.github.com",
            "name": "Chris Norman",
            "username": "chrnorm"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "819d08abb6311c5426993adc0290835aa43d47e6",
          "message": "client: reload in-cluster CA bundle on rotation (rustls-tls) (#1962)\n\nfeat(client): reload in-cluster CA bundle on rotation (rustls-tls)\n\nConfig::incluster() reads /var/run/secrets/kubernetes.io/serviceaccount/ca.crt\nonce and bakes the bytes into a RootCertStore. After CA rotation, new TLS\nhandshakes fail until the process restarts.\n\nTokenFile already re-reads the sibling token file in that same projected\nvolume every 60s. This adds the symmetric piece for ca.crt:\n\n- Config.root_cert_file: Option<PathBuf>, set by Config::incluster()\n- ReloadingVerifier: ServerCertVerifier that rebuilds an inner\n  WebPkiServerVerifier on a 60s timer, keeps stale roots on reload failure\n- rustls-tls only; openssl-tls unchanged\n\nConfig is now #[non_exhaustive] so this field addition (and future ones)\ndoesn't break downstream struct literals again.\n\nCloses #1953\n\nSigned-off-by: Chris Norman <chrisnorman@anthropic.com>\nSigned-off-by: Eirik A <sszynrae@gmail.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-05-04T19:37:53+01:00",
          "tree_id": "807cc1937055695abcd9e91b6c5282261b70f829",
          "url": "https://github.com/kube-rs/kube/commit/819d08abb6311c5426993adc0290835aa43d47e6"
        },
        "date": 1777919938812,
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
            "name": "Eirik A",
            "username": "clux"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "969a70e0e0fb1cb51a3527ee10a1ef88a9212fd9",
          "message": "Remove naked unwrap in new ReloadingVerifier (#1976)\n\nSigned-off-by: clux <sszynrae@gmail.com>",
          "timestamp": "2026-05-06T15:56:53+01:00",
          "tree_id": "b536c5c5fd899d0eb8d7c675139b1bddcce57981",
          "url": "https://github.com/kube-rs/kube/commit/969a70e0e0fb1cb51a3527ee10a1ef88a9212fd9"
        },
        "date": 1778079589293,
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
            "email": "SebTardif@ncf.ca",
            "name": "Sebastien Tardif",
            "username": "SebTardif"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fdaf064401ffa7294848b95911331701317d4071",
          "message": "Fix AttachedProcess task leak on drop and join() deadlock (#1978)\n\nTwo related issues in AttachedProcess:\n\n1. Dropping without calling join() or abort() detaches the background\n   task, which continues holding the WebSocket connection and sending\n   pings every 60 seconds indefinitely. Add a Drop impl that aborts\n   the task, matching the CancelableJoinHandle pattern used elsewhere\n   in kube-runtime.\n\n2. join() awaits the background task without first dropping the\n   DuplexStream halves (stdin_writer, stdout_reader, stderr_reader).\n   If stdout/stderr is enabled but the caller has not taken and\n   drained the reader, the background task blocks when the 1024-byte\n   DuplexStream buffer fills, while join() blocks waiting for the\n   task -- a deadlock. Fix by clearing all streams before awaiting,\n   matching the pattern in Portforwarder::join() which already does\n   this correctly.\n\nThe task field is changed to Option<JoinHandle> so that join() can\ntake ownership of the handle while still allowing Drop to abort it\nwhen join() is not called.\n\nSigned-off-by: Sebastien Tardif <sebtardif@ncf.ca>",
          "timestamp": "2026-05-11T14:42:13+01:00",
          "tree_id": "7fd6b6f320cc4738dc05b29c482494f81e988663",
          "url": "https://github.com/kube-rs/kube/commit/fdaf064401ffa7294848b95911331701317d4071"
        },
        "date": 1778507010256,
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
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "89284aade8c96f8c228fbecf437c457d3b6d87a5",
          "message": "Chore(deps): Update serde-saphyr requirement from 0.0.25 to 0.0.26 (#1979)\n\nUpdates the requirements on [serde-saphyr](https://github.com/bourumir-wyngs/serde-saphyr) to permit the latest version.\n- [Release notes](https://github.com/bourumir-wyngs/serde-saphyr/releases)\n- [Commits](https://github.com/bourumir-wyngs/serde-saphyr/compare/0.0.25...0.0.26)\n\n---\nupdated-dependencies:\n- dependency-name: serde-saphyr\n  dependency-version: 0.0.26\n  dependency-type: direct:production\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-11T14:43:54+01:00",
          "tree_id": "3045e47df9b4da712353fe33da63b235cecfb26e",
          "url": "https://github.com/kube-rs/kube/commit/89284aade8c96f8c228fbecf437c457d3b6d87a5"
        },
        "date": 1778507116938,
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
            "email": "SebTardif@ncf.ca",
            "name": "Sebastien Tardif",
            "username": "SebTardif"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "360c3db6fdf30368dcd2fd3df84d1d019f6acc9f",
          "message": "Handle RwLock write poisoning in ReloadingVerifier (#1977)\n\nPR #1976 fixed the read lock in ReloadingVerifier::current() to\ngracefully handle poisoning via unwrap_or_else, but the write lock\non line 156 was left as a bare .unwrap(). If any thread panics while\nholding the lock, subsequent CA bundle reload attempts cascade-panic\ninstead of recovering.\n\nApply the same unwrap_or_else(|e| e.into_inner()) pattern already\nused by the read lock five lines above.\n\nSigned-off-by: Sebastien Tardif <sebtardif@ncf.ca>",
          "timestamp": "2026-05-11T14:44:57+01:00",
          "tree_id": "61ac7304bb2808b77bdb812aae1b7165bd010e84",
          "url": "https://github.com/kube-rs/kube/commit/360c3db6fdf30368dcd2fd3df84d1d019f6acc9f"
        },
        "date": 1778507178295,
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
            "name": "Eirik A",
            "username": "clux"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2299d763fede95fd814bb39a133232f83fa887df",
          "message": "Drop failing stdin writer ws test (#1981)\n\n* Drop failing stdin writer ws test\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* link to issue\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n---------\n\nSigned-off-by: clux <sszynrae@gmail.com>",
          "timestamp": "2026-05-12T11:04:38+01:00",
          "tree_id": "59fbc1d89bf70521fe02c7684f2e381d09aff8c3",
          "url": "https://github.com/kube-rs/kube/commit/2299d763fede95fd814bb39a133232f83fa887df"
        },
        "date": 1778580360454,
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
          "id": "67e46715b4bf2697c83ffdac9ece022f7e45259b",
          "message": "Add CEL validation via kube-cel re-export (#1954)\n\n* feat(kube-core): add CEL validation via kube-cel re-export\n\nRe-export the kube-cel crate through kube-core::cel behind the `cel`\nfeature flag (Option D from #1670). This gives users access to CEL\ncompilation, validation, and all Kubernetes CEL extension functions\nvia `kube::core::cel::*`.\n\n- Convert `cel.rs` → `cel/mod.rs` with `pub use kube_cel::*` re-export\n- Add `kube-cel = \"0.4\"` optional dependency with `validation` feature\n- Propagate `cel` feature through kube umbrella crate\n- Add RUSTSEC-2024-0436 ignore in deny.toml (paste via cel, pending cel 0.13)\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(cargo): update kube-cel to version 0.5 and clean up deny.toml\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n---------\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-05-12T11:14:30+01:00",
          "tree_id": "e5289ccd6382d12f432c5e3aa4739e03b2311620",
          "url": "https://github.com/kube-rs/kube/commit/67e46715b4bf2697c83ffdac9ece022f7e45259b"
        },
        "date": 1778580952330,
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
          "id": "0f0cb937884ad88fd13c139d86ef12cb473327a2",
          "message": "Api<PartialObjectMeta<K>> should opportunistically degrade to metadata requests (#1952)\n\n* feat(kube-core): add metadata_api() to Resource trait\n\nAdd `fn metadata_api() -> bool` to the `Resource` trait (default false),\noverridden to return true for `PartialObjectMeta<K>`. This allows\ndownstream code to detect metadata-only types at compile time and\nautomatically switch to efficient metadata-optimized API requests.\n\nRef: #1614\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(kube-client): auto-use metadata headers for PartialObjectMeta\n\nApi methods get, list, watch, and patch now branch on\nResource::metadata_api() to automatically use metadata-optimized\nAccept headers when K = PartialObjectMeta<_>, so the API server\nreturns only metadata instead of the full object.\n\nRef: #1614\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* refactor(kube-runtime): deprecate metadata_watcher\n\nWith Api<PartialObjectMeta<K>> now automatically using metadata-only\nrequests, metadata_watcher is no longer needed. Users can use\nwatcher(Api::<PartialObjectMeta<K>>::all(client), config) instead.\n\nSimplify the dynamic_watcher example to remove the runtime branching\nand focus on dynamic resource watching.\n\nRef: #1614\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* test(kube-client): verify PartialObjectMeta uses metadata headers\n\nAdd mock-based tests verifying that Api<PartialObjectMeta<Pod>>\nsends the correct metadata-only Accept headers for get, list, watch,\nand patch operations.\n\nRef: #1614\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* refactor: cache metadata_api on Api to avoid tightening method bounds\n\nPR review feedback (clux): the original PR added `K: Resource` to\n`impl<K> Api<K>` in core_methods.rs (so it could call `K::metadata_api()`\nper request), which forced the same bound onto `ApiMode for FullObject<K>`\nand `ApiMode for MetaOnly<K>` in watcher.rs. This goes against the spirit\nof #1393 by tightening method-level bounds beyond what the corresponding\nApi/watcher constructors already require.\n\nCache the flag on `Api<K>` itself in the existing `impl<K: Resource> Api<K>`\nconstructors, then read `self.metadata_api` from the per-method code paths.\nThe Resource bound stays exactly where it was before this PR (Api\nconstructors and the public `watcher`/`metadata_watcher` signatures),\nno user-visible API change.\n\nSee https://github.com/kube-rs/kube/pull/1952#discussion_r3174134422\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n---------\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-05-12T15:01:41+01:00",
          "tree_id": "65c3fdf7641d0bb6e504b8ef20f766df24dd1d55",
          "url": "https://github.com/kube-rs/kube/commit/0f0cb937884ad88fd13c139d86ef12cb473327a2"
        },
        "date": 1778594561389,
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
            "email": "daniil.grigorev.dev@gmail.com",
            "name": "Danil Grigorev",
            "username": "Danil-Grigorev"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1f5a46b48cd69288ee3fa1423e8662237f7e60e9",
          "message": "fix and re-enable exec test (#1987)\n\nre-enable exec test with added wait on container readiness\n\nSigned-off-by: Danil-Grigorev <daniil.grigorev.dev@gmail.com>",
          "timestamp": "2026-05-24T07:52:51+01:00",
          "tree_id": "493269bd6f5f61fcc3ed43c48de711e3741dbce3",
          "url": "https://github.com/kube-rs/kube/commit/1f5a46b48cd69288ee3fa1423e8662237f7e60e9"
        },
        "date": 1779605640217,
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
          "id": "d291bd5f9d74ad3c151a5eef80485bb719560e53",
          "message": "feat(core): add AdmissionRequest::to_cel_request() for VAP CEL bridging (#1991)\n\n* feat(core): add AdmissionRequest::to_cel_request() for VAP CEL bridging\n\nBridges kube_core::admission::AdmissionRequest<T> to\nkube_cel::vap::AdmissionRequest behind the `cel` feature, so webhook\nhandlers can feed admission requests into client-side VAP evaluation.\n\nRefs kube-rs/kube-cel#4\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* fix(core): require kube-cel 0.5.1 for vap module\n\nThe vap module that to_cel_request() projects into was added in\nkube-cel 0.5.1; the prior \"0.5\" requirement allowed 0.5.0 and broke\nthe minimal-versions check.\n\nRefs kube-rs/kube-cel#4\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n---------\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>",
          "timestamp": "2026-05-27T15:51:15+01:00",
          "tree_id": "5655f2f3feddfeae9f47950c5310e32c70064e67",
          "url": "https://github.com/kube-rs/kube/commit/d291bd5f9d74ad3c151a5eef80485bb719560e53"
        },
        "date": 1779893619854,
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
            "email": "alexander.lvov.git@gmail.com",
            "name": "Alexander Lvov",
            "username": "Alvov1"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b442d5885f166d21b6d93e6f383b5ffb1440af7b",
          "message": "feat(runtime): implement Store::state_with and Store::state_filtered (#1998)\n\n* feat(runtime): implement Store::state_with and Store::state_filtered\n\nAdds two new methods on Store<K> for filtering the local cache:\n- state_with(predicate) for arbitrary predicate-based filtering\n- state_filtered(selector) as a convenience wrapper using label Selector\n\nCloses #970\n\nSigned-off-by: Alexander Lvov <alexander.lvov.git@gmail.com>\n\n* docs(runtime): add doc-test for state_filtered\n\nSigned-off-by: Alexander Lvov <alexander.lvov.git@gmail.com>\n\n---------\n\nSigned-off-by: Alexander Lvov <alexander.lvov.git@gmail.com>",
          "timestamp": "2026-06-09T22:55:45+01:00",
          "tree_id": "3c2586484ef19fff23f2c69b047773e25b214ead",
          "url": "https://github.com/kube-rs/kube/commit/b442d5885f166d21b6d93e6f383b5ffb1440af7b"
        },
        "date": 1781042275174,
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
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b07d250ad289fa2cd896aa0d9749b64fdff5c536",
          "message": "Chore(deps): Update serde-saphyr requirement from 0.0.26 to 0.0.27 (#1992)\n\nUpdates the requirements on [serde-saphyr](https://github.com/bourumir-wyngs/serde-saphyr) to permit the latest version.\n- [Release notes](https://github.com/bourumir-wyngs/serde-saphyr/releases)\n- [Commits](https://github.com/bourumir-wyngs/serde-saphyr/compare/0.0.26...0.0.27)\n\n---\nupdated-dependencies:\n- dependency-name: serde-saphyr\n  dependency-version: 0.0.27\n  dependency-type: direct:production\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>\nCo-authored-by: doxxx <doxxx93@gmail.com>",
          "timestamp": "2026-06-11T00:55:57Z",
          "tree_id": "b7d1ca279f41ecc779eb38c86eb968c8ac430cd4",
          "url": "https://github.com/kube-rs/kube/commit/b07d250ad289fa2cd896aa0d9749b64fdff5c536"
        },
        "date": 1781139409554,
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
            "email": "info@orangecms.org",
            "name": "Daniel Maslowski",
            "username": "orangecms"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "3103370bbca96d5cc56c78f668c40175f00f98c9",
          "message": "feat(runtime): add wait::conditions::is_created helper (#2000)\n\nThis is the counterpart to is_deleted.\n\nSigned-off-by: Jiji Freya Daniel Maslowski <info@orangecms.org>",
          "timestamp": "2026-06-11T16:37:14+01:00",
          "tree_id": "277a9fb4dff94743919d0255bf4e9ecdbfa7dd8c",
          "url": "https://github.com/kube-rs/kube/commit/3103370bbca96d5cc56c78f668c40175f00f98c9"
        },
        "date": 1781192419010,
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
            "email": "aviram@metalbear.com",
            "name": "Aviram Hassan",
            "username": "aviramha"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ec8190f3960e0130bad03bf6312fc44edcc532a2",
          "message": "support auth exec yaml output (#2003)\n\nSigned-off-by: Aviram Hassan <aviram@metalbear.com>",
          "timestamp": "2026-06-11T23:33:04+01:00",
          "tree_id": "0bf5084db205f27d6d175d37a90e818afe77ed73",
          "url": "https://github.com/kube-rs/kube/commit/ec8190f3960e0130bad03bf6312fc44edcc532a2"
        },
        "date": 1781217255920,
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
            "email": "84204691+dgunzy@users.noreply.github.com",
            "name": "dgunzy",
            "username": "dgunzy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b64ad83edbd7ef3c1f2148495d1b5431d1db88f4",
          "message": "fix(client): apply tls-server-name on the openssl-tls path (#1993)\n\n* fix(client): apply tls-server-name on the openssl-tls path\n\nSigned-off-by: Daniel Guns <danbguns@gmail.com>\n\n* test(client): cover IP tls-server-name and accept_invalid_certs paths\n\nSigned-off-by: Daniel Guns <danbguns@gmail.com>\n\n---------\n\nSigned-off-by: Daniel Guns <danbguns@gmail.com>",
          "timestamp": "2026-06-12T23:51:03+01:00",
          "tree_id": "a144976b94dbee6daa27c58463f0544991bfd250",
          "url": "https://github.com/kube-rs/kube/commit/b64ad83edbd7ef3c1f2148495d1b5431d1db88f4"
        },
        "date": 1781304765071,
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
            "email": "alexander.lvov.git@gmail.com",
            "name": "Alexander Lvov",
            "username": "Alvov1"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6c266576bc847c75bc0de9edb713fe73d934f745",
          "message": "refactor(runtime): rename Store::state_with/state_filtered per review feedback (#2002)\n\n* refactor(runtime): rename Store::state_with/state_filtered per review feedback\n\n- state_with -> state_filter (mirrors Iterator::filter naming)\n- state_filtered -> state_filter_selector\n- add doc note that read lock is held during the predicate evaluation\n\nSigned-off-by: Alexander Lvov <alexander.lvov.git@gmail.com>\n\n* refactor(runtime): use is_some_and in store test predicate\n\nSigned-off-by: Alexander Lvov <alexander.lvov.git@gmail.com>\n\n---------\n\nSigned-off-by: Alexander Lvov <alexander.lvov.git@gmail.com>",
          "timestamp": "2026-06-13T13:02:34+01:00",
          "tree_id": "64b4c0189673bbb4b0967b1d91769df1a9db69e0",
          "url": "https://github.com/kube-rs/kube/commit/6c266576bc847c75bc0de9edb713fe73d934f745"
        },
        "date": 1781352260852,
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
          "id": "c261d787af68af60616bcde3ffac5d2ee51297f9",
          "message": "deps: bump kube-cel to 0.6.1 (validation surface flattened) (#2005)\n\nkube-cel 0.6 flattened its validation surface: the `vap` module is now\nprivate and its types (`AdmissionRequest`, `GroupVersionKind`,\n`GroupVersionResource`) are re-exported flat at the crate root. Update\n`AdmissionRequest::to_cel_request` to use the new flat paths.\n\nBREAKING CHANGE (kube-core `cel` surface): `to_cel_request` now returns\n`kube_cel::AdmissionRequest` instead of `kube_cel::vap::AdmissionRequest`,\nand the module-qualified paths `kube::core::cel::vap::*` /\n`...::compilation::*` are gone in 0.6. The flat type names re-exported\nthrough `kube_core::cel` keep working.\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>",
          "timestamp": "2026-06-13T15:23:45+01:00",
          "tree_id": "35525502e77424a0ff167c8edbd7f48d93b695a4",
          "url": "https://github.com/kube-rs/kube/commit/c261d787af68af60616bcde3ffac5d2ee51297f9"
        },
        "date": 1781360723444,
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
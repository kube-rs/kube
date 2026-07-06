window.BENCHMARK_DATA = {
  "lastUpdate": 1783336440568,
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
          "id": "3dd76bc096be66bb6d90dfd269fddc6ff1e81a2c",
          "message": "bump k8s-openapi to 0.28 (#2009)\n\n* bump k8s-openapi to 0.28\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* actually bump\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* minimal version syntax\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* minor breaking changes in TokenRequest / TokenReview\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* bump more refs\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* find one more version failing to bump\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n---------\n\nSigned-off-by: clux <sszynrae@gmail.com>",
          "timestamp": "2026-06-15T15:02:02+01:00",
          "tree_id": "863669a1badd0aa561e2d58895b33db77880faee",
          "url": "https://github.com/kube-rs/kube/commit/3dd76bc096be66bb6d90dfd269fddc6ff1e81a2c"
        },
        "date": 1781532195722,
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
          "id": "3dd76bc096be66bb6d90dfd269fddc6ff1e81a2c",
          "message": "bump k8s-openapi to 0.28 (#2009)\n\n* bump k8s-openapi to 0.28\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* actually bump\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* minimal version syntax\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* minor breaking changes in TokenRequest / TokenReview\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* bump more refs\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* find one more version failing to bump\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n---------\n\nSigned-off-by: clux <sszynrae@gmail.com>",
          "timestamp": "2026-06-15T15:02:02+01:00",
          "tree_id": "863669a1badd0aa561e2d58895b33db77880faee",
          "url": "https://github.com/kube-rs/kube/commit/3dd76bc096be66bb6d90dfd269fddc6ff1e81a2c"
        },
        "date": 1781532449651,
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
          "id": "a5b4f3fb9729d7317a3f4521a37481193772a19c",
          "message": "Box a large runtime error in ReconcilerErr (#1880)\n\n* Box some runtime erorrs for clippy\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* remove misleading _ prefix in example\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n* make it clear what allow directive goes away\n\nSigned-off-by: clux <sszynrae@gmail.com>\n\n---------\n\nSigned-off-by: clux <sszynrae@gmail.com>\nSigned-off-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-06-15T17:30:49+01:00",
          "tree_id": "eb1a90c5ec7ac65049e6d8f1b8399cbfe69ec3f2",
          "url": "https://github.com/kube-rs/kube/commit/a5b4f3fb9729d7317a3f4521a37481193772a19c"
        },
        "date": 1781541474166,
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
          "id": "8d617848ef5095fe0798bdc94f6b6f1a2245ce65",
          "message": "Enable `RetryPolicy::server_retry` by default for `Client` (#2007)\n\n* Enable RetryPolicy::server_retry by default for Client\n\nSigned-off-by: Danil-Grigorev <daniil.grigorev.dev@gmail.com>\n\n* switch form with_retry to default_retry\n\nSigned-off-by: Danil-Grigorev <daniil.grigorev.dev@gmail.com>\n\n---------\n\nSigned-off-by: Danil-Grigorev <daniil.grigorev.dev@gmail.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-06-16T09:23:12Z",
          "tree_id": "eb74174ada856b9f57bc8d1871c9fec78c2c622a",
          "url": "https://github.com/kube-rs/kube/commit/8d617848ef5095fe0798bdc94f6b6f1a2245ce65"
        },
        "date": 1781601859840,
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
          "id": "2310a068599f8d2d94336ebc491b14376dbd4dd9",
          "message": "feat(derive): client-side CEL validation via #[kube(cel)] / #[x_kube(cel)] (#2011)\n\n* feat(derive): client-side CEL validation via #[kube(cel)] / #[x_kube(cel)]\n\nGenerate client-side CEL validation methods so users can evaluate the same\nx-kubernetes-validations rules locally, without an apiserver (issue #1670).\n\n- `#[kube(cel)]` on a CustomResource generates `Foo::validate_cel(&self)`\n  (creation rules) and `Foo::validate_cel_update(&self, old)` (transition rules).\n  Forces the KubeSchema derive; rejected at compile time with `schema = \"manual\"`\n  since a manual schema carries no validations.\n- `#[x_kube(cel)]` on any KubeSchema struct generates a static\n  `T::validate_cel(value, old)` usable on a serialized fragment in unit tests.\n\nThe sub-struct method regenerates the schema with the same openAPIV3 settings and\nstructural transforms the CRD path uses, so the schema kube-cel walks matches what\nan apiserver would validate (`schemars::schema_for!` alone is not walkable).\nValidation is per-call (Validator::new().validate); caching can follow.\n\nRequires the downstream crate to enable `kube/cel`, since the generated code\nreferences `kube::core::cel`.\n\nAdds runtime integration tests, a compile-fail test for `schema = \"manual\"` + cel,\na doctest, and the `crd_derive_cel` example.\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* feat(derive): validate_cel returns Result<(), ValidationErrors>\n\nAddress review feedback (#2011): returning `Vec<ValidationError>` forced callers\nto check `is_empty()`, inverting the idiomatic `validate()?` flow.\n\nkube-cel 0.7.0 flipped its validation entry points to `Result<(), ValidationErrors>`,\nso the generated `validate_cel` / `validate_cel_update` (root) and the static\nsub-struct `validate_cel` now mirror that: `Ok(())` on success, the aggregated\nfailures otherwise. Bumps the kube-cel floor 0.6.1 -> 0.7.0 (kube-core).\n\nUpdates the doctest, the crd_derive_cel example, and the integration tests to\n`is_ok()` / `is_err()`.\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* refactor(derive): delegate CEL validation bodies to kube-core helpers\n\nMove the client-side CEL validation logic out of the proc-macro output and\ninto kube-core free functions, so it is compiled once instead of being\nre-expanded (and re-parsed) at every derive site.\n\n- kube-core::cel::{validate_cel, validate_cel_update} (cfg(cel)): read the\n  schema from <T as CustomResourceExt>::crd(), so no schemars at runtime and\n  no schema feature needed.\n- kube-core::cel::validate_cel_schema (cfg(all(cel, schema))): runs the\n  schemars openapi3 settings + kube_core::schema transforms for the\n  #[x_kube(cel)] sub-struct path.\n\nThe derives still emit the same inherent methods, now one-line delegations,\nso the macro stays the opt-in gatekeeper and the schema = \"manual\"\ncompile_error guard is unchanged. Call-site feature requirements are\nidentical to the previous inline bodies.\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n---------\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>",
          "timestamp": "2026-06-16T16:43:47+01:00",
          "tree_id": "5353da7c055468204464605a6af09cd1aae056e4",
          "url": "https://github.com/kube-rs/kube/commit/2310a068599f8d2d94336ebc491b14376dbd4dd9"
        },
        "date": 1781624710545,
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
          "id": "b4f0cc4d7b4ce00ac7fa2d85c0120bbd38fa6210",
          "message": "re-hook cel feature and move dev-dep pin\n\nSigned-off-by: clux <sszynrae@gmail.com>",
          "timestamp": "2026-06-16T18:29:01+01:00",
          "tree_id": "88af9c118fb09d1bfebe176f6f31be77f4709330",
          "url": "https://github.com/kube-rs/kube/commit/b4f0cc4d7b4ce00ac7fa2d85c0120bbd38fa6210"
        },
        "date": 1781631065915,
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
          "id": "f065f91ddac4d8b36a1eae1816200967aff904df",
          "message": "add support for https proxy (#2013)\n\nSigned-off-by: goenning <me@goenning.net>",
          "timestamp": "2026-06-18T10:38:56+01:00",
          "tree_id": "ce5d3ad69a04028a36743d1efac52ea1eb3cb3c4",
          "url": "https://github.com/kube-rs/kube/commit/f065f91ddac4d8b36a1eae1816200967aff904df"
        },
        "date": 1781775633688,
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
          "id": "4dcfd94c1c0c886abf0bc6f432563bb25888d2ab",
          "message": "fix cargo hack on https proxy feature (#2014)\n\nSigned-off-by: goenning <me@goenning.net>",
          "timestamp": "2026-06-18T14:30:39+01:00",
          "tree_id": "edc8b7253f3d167f66a22fe31d50c074a121ce8b",
          "url": "https://github.com/kube-rs/kube/commit/4dcfd94c1c0c886abf0bc6f432563bb25888d2ab"
        },
        "date": 1781789539676,
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
          "id": "a795e7e925eca034d43ad519b446548bad44db64",
          "message": "Chore(deps): Update kube-cel requirement from 0.7.0 to 0.8.0 (#2017)\n\nUpdates the requirements on [kube-cel](https://github.com/kube-rs/kube-cel) to permit the latest version.\n- [Release notes](https://github.com/kube-rs/kube-cel/releases)\n- [Changelog](https://github.com/kube-rs/kube-cel/blob/main/CHANGELOG.md)\n- [Commits](https://github.com/kube-rs/kube-cel/compare/v0.7.0...v0.8.0)\n\n---\nupdated-dependencies:\n- dependency-name: kube-cel\n  dependency-version: 0.8.0\n  dependency-type: direct:production\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-06-22T08:22:10+01:00",
          "tree_id": "41401cbda46593f4cf1eba060969b42f00382083",
          "url": "https://github.com/kube-rs/kube/commit/a795e7e925eca034d43ad519b446548bad44db64"
        },
        "date": 1782113050743,
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
            "email": "git@vini.cat",
            "name": "Vinicius Deolindo",
            "username": "iniw"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c983e5b6c5574572a0af1c6e22d2a9031d29607e",
          "message": "client: include sources in `CommitError`'s error message (#2020)\n\n# Context\n\nA user report left us with logs that essentially boiled down to a `CommitError` message:\n\n> failed to save object\n\nThat made the failure look generic and sent me down a couple-hour long debug session.\n\nThe actual cause was pretty simple, and once I eventually managed to reproduce it I saw that the inner `kube::Error` contained a pretty clear message for the actual problem and I thought \"Damn, having that information would've made this so much easier\" - so this change is to help future me :)\n\n# Summary\n\nThis PR just appends the existing error messages with `: {0}`, which is a pattern already present in other places such as the top-level `kube::Error` type.\n\nSigned-off-by: Vinicius Deolindo <git@vini.cat>",
          "timestamp": "2026-06-26T11:40:16+01:00",
          "tree_id": "67eadaffddba260583256f95a28448b08dce295d",
          "url": "https://github.com/kube-rs/kube/commit/c983e5b6c5574572a0af1c6e22d2a9031d29607e"
        },
        "date": 1782470533325,
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
          "id": "fb5f04d65a088e71b713a4f60071be1b2aac807a",
          "message": "Include error sources in Display messages at client, core, runtime (#2021)\n\nclient, core, runtime: include error sources in Display messages\n\nSeveral error variants carried a #[source]/#[from] field that the\n#[error(...)] message never interpolated, so the underlying cause was\ndropped from plain Display/log output and only visible by walking\n.source(). Append \": {0}\" to surface it, matching the convention the\ntop-level kube::Error already follows.\n\nNote: controller::Error and runner::Error now require Display on their\ngeneric params for the generated Display impl (already implied by the\nexisting #[source] Error bound).\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>",
          "timestamp": "2026-06-26T23:42:01+01:00",
          "tree_id": "6b3c0152c12ef83673cdb7376ada868e63522645",
          "url": "https://github.com/kube-rs/kube/commit/fb5f04d65a088e71b713a4f60071be1b2aac807a"
        },
        "date": 1782513766721,
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
          "id": "f0937801d56def7938d2e8a6814b573475a463cd",
          "message": "chore(ci): widen clippy to all targets, fix surfaced lints (#2023)\n\n* chore(ci): widen clippy to all targets, fix surfaced lints\n\n`just clippy` (and the CI job that now calls it) skipped test/example/bench\ncode (no --all-targets) and only ran --all-features on the kube facade.\nWiden to two passes, with no hardcoded feature list so features added later\nare linted automatically:\n\n    cargo clippy --workspace --all-features --all-targets --exclude e2e\n    cargo clippy --workspace --all-targets\n\ne2e is excluded from the --all-features pass because it enables both\nk8s-openapi `latest` and `mk8sv`, panicking the build script; the default\npass lints it and the `#[cfg(not(feature))]` paths instead.\n\nFixes the lints this surfaced in previously-unlinted code:\n- kube-client: box ProviderToken::Oidc (large_enum_variant); the enum is\n  private, so this is not a public API change\n- kube-core: drop a redundant `&` in a panic! arg\n- kube-runtime: drop redundant `as u64` casts; vec! for a >16KB test array\n- allow(dead_code) on intentional test/example fixtures\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n* chore(ci): lint via rs-clippy-check action, not `run: just clippy`\n\nKeep the clippy action so it still annotates the diff (it's good at getting\ncontributors to fix lints unprompted), running both `just clippy` passes\nthrough it. Gating is unchanged: a non-zero clippy exit (deny-level lint)\nfails the job, warnings stay advisory.\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>\n\n---------\n\nSigned-off-by: doxxx93 <doxxx93@gmail.com>",
          "timestamp": "2026-06-27T14:38:38+01:00",
          "tree_id": "504d2016ebd934bee60b9d645ffdb6618dc78e5b",
          "url": "https://github.com/kube-rs/kube/commit/f0937801d56def7938d2e8a6814b573475a463cd"
        },
        "date": 1782567573625,
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
            "email": "bojidar.marinov.bg@gmail.com",
            "name": "Bojidar Marinov",
            "username": "bojidar-bg"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8e2c23ea2083dab5f2a6889796711bb0b47e921a",
          "message": "Remove extraneous Sync bound in Controller::reconcile_all_on (#2029)\n\nSigned-off-by: Bozhidar Marinov <bozhidar.marinov1@digits.schwarz>",
          "timestamp": "2026-07-01T10:37:13+01:00",
          "tree_id": "e411a594a9653b93a167b4928e344e2c05385558",
          "url": "https://github.com/kube-rs/kube/commit/8e2c23ea2083dab5f2a6889796711bb0b47e921a"
        },
        "date": 1782898756792,
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
          "id": "ae49cce192b85db3d734d290a6031aa2d9ac60e0",
          "message": "client: fall back to OS-native cert store via rustls-platform-verifier (#2030)\n\n* client: fall back to OS-native cert store via rustls-platform-verifier\n\nWhen no certificate-authority is configured in the kubeconfig, client-go\ndefers TLS verification to the platform trust store. Add an opt-in\nrustls-platform-verifier feature that does the same for the rustls\nbackend, using the OS-native verifier (macOS Security framework, Windows\nCryptoAPI, native roots on Linux) instead of the webpki/native-roots\nfallback. Behaviour is unchanged by default and when a CA is configured.\n\nCloses #2028\n\nSigned-off-by: Aviram Hassan <aviram@metalbear.com>\n\n* make platform verifier the default rustls no-CA path\n\nFold rustls-platform-verifier into the rustls-tls feature and use it by\ndefault when no CA is configured, replacing the with_native_roots()\nfallback (which it supersedes, including removing the Android/iOS panic).\nwebpki-roots remains an explicit opt-in override for the bundled Mozilla\nroots.\n\nSigned-off-by: Aviram Hassan <aviram@metalbear.com>\n\n* reuse AddRootCertificate variant for platform verifier errors\n\nKeep the public rustls_tls::Error enum unchanged by mapping the\nrustls::Error from with_platform_verifier() into the existing\ntype-erased AddRootCertificate variant, and restore NoValidNativeRootCA\nrather than removing it.\n\nSigned-off-by: Aviram Hassan <aviram@metalbear.com>\n\n* cargo-deny: allow CDLA-Permissive-2.0 for webpki-root-certs\n\nPulled in transitively via rustls-platform-verifier, now the default\nno-CA trust source on the rustls-tls stack.\n\nSigned-off-by: Aviram Hassan <aviram@metalbear.com>\n\n---------\n\nSigned-off-by: Aviram Hassan <aviram@metalbear.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-07-02T15:39:48+01:00",
          "tree_id": "3ee94e5ffd6fa76dc544e651b8fea56651f6f45c",
          "url": "https://github.com/kube-rs/kube/commit/ae49cce192b85db3d734d290a6031aa2d9ac60e0"
        },
        "date": 1783003249606,
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
          "id": "0bcb4c78e9c8387963c49f07e5e7742f6487c028",
          "message": "bump msrv to 1.89 (#2035)\n\nbump msrv\n\nSigned-off-by: clux <sszynrae@gmail.com>",
          "timestamp": "2026-07-06T11:46:16+01:00",
          "tree_id": "94c013dcd33fdbce8596a5540c99a530ccb9b439",
          "url": "https://github.com/kube-rs/kube/commit/0bcb4c78e9c8387963c49f07e5e7742f6487c028"
        },
        "date": 1783334857045,
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
          "id": "660966eb316a16a31fcb6a2233176438829bb505",
          "message": "Chore(deps): Update serde-saphyr requirement from 0.0.27 to 0.0.29 (#2032)\n\nUpdates the requirements on [serde-saphyr](https://github.com/bourumir-wyngs/serde-saphyr) to permit the latest version.\n- [Release notes](https://github.com/bourumir-wyngs/serde-saphyr/releases)\n- [Commits](https://github.com/bourumir-wyngs/serde-saphyr/compare/0.0.27...0.0.29)\n\n---\nupdated-dependencies:\n- dependency-name: serde-saphyr\n  dependency-version: 0.0.29\n  dependency-type: direct:production\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>\nCo-authored-by: Eirik A <sszynrae@gmail.com>",
          "timestamp": "2026-07-06T12:12:17+01:00",
          "tree_id": "93b5ea2a8cf76f42b037a17d69a2c22a3543e41c",
          "url": "https://github.com/kube-rs/kube/commit/660966eb316a16a31fcb6a2233176438829bb505"
        },
        "date": 1783336438599,
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
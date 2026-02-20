window.BENCHMARK_DATA = {
  "lastUpdate": 1771548270051,
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
      }
    ]
  }
}
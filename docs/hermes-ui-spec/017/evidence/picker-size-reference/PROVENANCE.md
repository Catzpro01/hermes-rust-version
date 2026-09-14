## Tested proposal (cycle 12 GREEN)

`tested-terminal-size.patch` is the tree the runner verified in
`Picker diagnostic regression` run
[34880104897](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34880104897)
(job 104099602199, commit `b6455ee`): the bound proposal after the job's
`cargo fmt --all`, exported by the `Export exactly tested proposal` step.

- sha256 `f74411bf591844cf3df3cb0df8a41b99b1edb5c59997d625d77d90614dabfddb`, 5178 bytes
- the only difference from the bound patch (digest `940c0809…`, 5141 bytes) is
  `rustfmt` wrapping the final `assert_eq!` in the new unit test
- live verification: `green actual CLI regression verified three times`,
  `rust picker-narrow 40 CAPTURED_NOT_REVIEWED`,
  `rust picker-too-small 39 CAPTURED_NOT_REVIEWED`, trace digest
  `sha256=bd1213c3…; bytes=5139`

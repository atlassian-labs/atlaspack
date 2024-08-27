# lmdb-js-lite benchmarks

Simple benchmarks of the NAPI interface against lmdb-js.

## Usage

```
for i in __benchmarks__/*.ts ; do printf "$i\n=========================\n"; yarn ts-node $i; done
```

```
for i in __benchmarks__/*.ts ; do printf "$i\n=========================\n"; yarn ts-node $i; done 
```
# the result analysis of realjump

## 1. Introduction
there are two types of mapping in the results: weighted and ame bank. the weighted mapping sorted the whole graph and evenly spread them in to each subarray. the same bank first distribute same amount of jubs in each bank. and distribute these jobs in to each subarray interleave.

for each mapping, there are 5 jummping method:


the original data: [link](https://docs.google.com/spreadsheets/d/1SNVbOT9f0KC9yKf-ESHL8fR4347Uq85e1Wl1C3D2bmU/edit?usp=sharing)
1. the normal jumping:  directly jump to the target location from current location
```rust
  fn update(&mut self, evil_row_status: (usize, usize), location: &RowLocation, size: usize) {
        let current_col = evil_row_status.1;
        let target_col = location.col_id.0;
        let jumps = (current_col as isize - target_col as isize).abs() as usize;
        // the jump of size
        if jumps > 4 {
            self.jump_multiple_cycle += jumps;
        } else {
            self.jump_one_cycle += jumps;
        }
        self.jump_one_cycle += (size - 1) * 4;
    }
```
2. ideal jump: no jump overhead!
```rust
 fn update(&mut self, _evil_row_status: (usize, usize), _location: &RowLocation, size: usize) {
        self.total_cycle += (size - 1) * 4;
    }
```

3. from source jump: always jump from location 1!
```rust
 fn update(&mut self, _evil_row_status: (usize, usize), location: &RowLocation, size: usize) {
        if location.col_id.0 > 4 {
            self.jump_multiple_cycle += location.col_id.0;
        } else {
            self.jump_one_cycle += location.col_id.0;
        }
        self.jump_one_cycle += (size - 1) * 4;
    }
```

4. Myjump: currently it's identical to ideal jump
5. smart jump: combine the from source jump and normal jump, always choose the quicker one:
```rust
fn update(&mut self, evil_row_status: (usize, usize), location: &RowLocation, size: usize) {
        let current_col = evil_row_status.1;
        let target_col = location.col_id.0;
        let jumps = (current_col as isize - target_col as isize).abs() as usize;
        let jumps = jumps.min(target_col);
        // the jump of size
        if jumps > 4 {
            self.jump_multiple_cycle += jumps;
        } else {
            self.jump_one_cycle += jumps;
        }
        self.jump_one_cycle += (size - 1) * 4;
    }
```

## the influence of the mapping
### the total simulation cycle
the average speed up of weighted mapping is 1.400722749.

#### case analysis
- the graph 2:"mtx/gearbox/soc-orkut.mtx",
  - speed up for row, col, dispacing: 1.164866208 0.8317156836 10.02369333
  - the col is slower, but the col cycle is the main bottle neck.
  - check the col cycle details:! FIND a bug here, the ideal is not correct,! there should be atleast one jump when the jump is 1!! rerun the tests but we can find the col is slow! the normal and fromssource is very slow. the smart is better. for
- the graph 3: evil is very slow, even the smart, but evil dinn't count to much in cycle. the normal is the the larges,

- the graph -3: jump is not main overhead. smart improves alot.
## the influence of each stage: row read, evil_row read, dispatching, col write
in the bencharks; 4 benchmark , the col is the bottlenet

8 benchmark: the row is the bottleneck.

non seems the dispaching to be the bottleneck.

## the split view of jump:
in col cycles: jump multi dominates the partion

in row cycles: 4 graphs jump one dominates, 7 graphs jump muilti dominates
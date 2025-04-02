# Transaction Engine

## Assumptions

- My understanding from the assignment wording, is that _**only deposit transactions can be disputed**_. So resolve and chargeback are also referring to deposits.\
  The documentation uses explicitly term like `increase` for `held` and `decrease` for `available` when discussing disputes.\
  Although it's never written anywhere that `amount` must be positive, the usage of both `increase` and `decrease` seems to suggest it is.\
  Also, it does not make sense for `held` funds to be negative.\
  For this reason I treat as an error if a `withdrawal` is disputed.
- Even though it's not written explicitly, I reject disputes for amounts that are greater than available funds.
- Documentation doesn't say if a dispute can refer to the wrong client for that transaction, so I check this explicitly.\
  If a `dispute`, `resolve` or `chargeback` refer to a transaction not belonging to the specified client, I ignore them.
- I made the following assumptions in case a resolve fails when freeing the funds: the tx remains under dispute.
- I am assuming there is always a third comma for `dispute`, `resolve` and `chargeback`, so the csv file must have a fixed format
- Precision: the documentation states it can be assumed a precision of 4 places past the decimal,
  but to be safe I am truncating Decimal when reading from the CSV (rounded using Bankers rounding).
  However, because addition and subtraction cannot change the initial precision, I am not truncating Decimal in the output.

## Scoring

### Completeness

It should cover all the cases.

### Correctness

There is a sufficient number of unit tests. Functions are kept short and with few responsibilities, so they can be tested.\
I split also the code for reading and writing to CSV, so many corner cases could be tested without resorting to 
comprehensive test files.\
By the time everything was put together there were basically no errors.

There is a small input file for testing (`input.csv`), that can be fed to the program and its output compared with
`expected.csv`.\
The program run as instructed

```shell
cargo run -- input_file > output_file
```

Also

```shell
cargo test
```

to run all the unit tests.

Even though the types serialized and deserialized with `serde` are using the type system, they have been tested with unit
tests as well.

### Safety and Robustness

Nothing dangerous. I am using `anyhow` for error reporting. Nothing should be panicking and all `Result`
are checked and propagated up to `main` where errors are printed to `stderr` so it does not interfere with the results.

Also, I tried the number of external crates to a minimum: besides the recommended `csv` and `serde`, I used only
`rust_decimal` and `anyhow`.

### Efficiency

I believe efficiency is overall decent. O(1) data structures, CSV file is read efficiently (at least according to
`csv` crate documentation). However, spatial efficiency is a concern and I decided to go with an easy solution and
maybe describe here potential alternatives and why I did not go for them.

So, the main issue is keeping track of all the transactions (well, not all, because I do not track `withdrawal`).\
For simplicity I track them in a HashMap, but billions of deposits will end up in a hashmap of many GBs.\
The reason I track them it's because `dispute` and the likes, refer to them.
In a real scenario there would be a timeframe for disputing things, so old transactions could be discarded (for the
purpose of this program).\
Another approach to keep them in memory would be to trim the hashmap in order to keep it within N elements, and this
could be done easily with a linked hashmap. But there is not such option for this assignment.\
Also, keeping everything in memory exposes everything to the risk of crashes, so in a real scenario I would have used
something more persistent, like Redis and then a few tweaks to optimize the performance due to the latency added
by the network connection.

Bottom line, with all the above said, I believe the solution hinted when pointing out that there can be many transactions
to store, was to use an embedded DB, but I felt more comfortable with the hashmap solution because there were many other
things to cover, and it would not have been within the requested time limits.

Also, if this was a server receiving CSVs from multiple TCP streams, I would keep the architecture as is (single-threaded)
and put a "sorting aggregator" in from of the transaction engine, so it will ensure that transactions are sent
based on their transaction order.

### Maintainability and readability

This has been my top priority. The code should be easy enough to read and change.\
I also used documentation almost everywhere.

## What I did and what I didn't

- I used Decimal for `amount` so calculations are accurate and without any loss.
- The design is pretty simple: a single threaded transaction engine. No async, no multiple threads.
- `total = available + held` is an invariant, but I am not exploiting this (e.g. every transaction can then be checked for internal consistency), but surely an improvement
- Not sure how I should have used the information that `tx` are not necessarily ordered, given instructions also say that transactions occur chronologically in the file.
- I am not using a fancy logger: printing to stdout for CSV results, printing to stderr for anything else.

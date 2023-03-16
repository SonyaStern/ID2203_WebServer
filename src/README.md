# Documentation

In this file I added some textual documentation about my contributions. 📖

First of all, I implemented some new libraries from **omnipaxos** and so to take the {Acceptor, Ballot, Paxos, Proposer}. Additionally I added the **HashMap** class from the **collections** class. Finally I added tree more classes so to use them for the output to a txt file namely: **fs::File**, **io::{BufWriter, Write}** and last but not least the **path::Path** class. All these are implemented in the **use** section of the **_main_** class.

## Function

I created a function called **<fn get_statistics\>** in this function I passed two variables namely **paxos** and **output_file_path**.

First this function takes a reference to a **Paxos** instance and returns a **HashMap** that maps **Ballot objects** to the number of acceptors that have accepted that ballot. The function iterates over each acceptor in the Paxos instance, retrieves the accepted ballot using <snap style="color:orange">**_get_accepted_ballot_**</snap>, and increments the count for that ballot in the **statistics HashMap**. If a ballot has not been accepted by any acceptor, it is not included in the HashMap.

```Rust
    for acceptor in &paxos.acceptors {
        let accepted_ballot = acceptor.get_accepted_ballot();
        if let Some(ballot) = accepted_ballot {
            *statistics.entry(ballot).or_insert(0) += 1;
        }
    }
```

The function writes the _statistics_ to this file (**output_file_path**) using a <snap style="color:orange">**BufWriter**</snap>.

Who we write on the file ✍️

```Rust
let path = Path::new(output_file_path);
    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);

    writeln!(&mut writer, "{:20} {}", "Ballot", "Count")?;
    for (ballot, count) in &statistics {
        writeln!(&mut writer, "{:20} {}", format!("{:?}", ballot), count)?;
    }

```

## Main

In the main we created a **Paxos instance** with three acceptors and then calls the <snap style="color:orange">**get_statistics**</snap> function to get statistics about these nodes. Also we pass the specified output file path: <snap style="color:olive">**let output_file_path**</snap>. This will create or overwrite the file with the given name.

```Rust
let output_file_path = "statistics.txt";
get_statistics(&paxos, output_file_path);
```

## Output Example: 👇

The output is formatted with a pretty table, using a fixed width for the ballot column.

|     Ballot      | Count |
| :-------------: | :---: |
| Ballot(3, 2, 1) |   3   |
| Ballot(3, 2, 1) |   1   |

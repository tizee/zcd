# fuzzy

A fuzzy algorithm for sorting the string list according to the target string.

The algorithm is derived from `fzy` as we only need to care about paths of directories.

```rust
use fuzzy::Matcher;

int main(){
 let is_match = Matcher::has_match("abc","/a/b/c");
 let fzy = Matcher::fzy;
 let match_score = fzy.match_score("abc", "/a/b/c")
}
```

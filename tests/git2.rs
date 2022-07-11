use ::{git2::Repository, save::git2::RepositoryExt};

// TODO: a helper that takes a repository, and a callback list of commands, and produces
// output showing the last commits, or the graph, or whatever.

#[test]
fn cli() {
    let _repo = Repository::temporary();
}

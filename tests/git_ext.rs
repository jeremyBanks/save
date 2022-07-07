use ::{git2::Repository, save::git_ext::RepositoryExt};

#[test]
fn cli() {
    let _repo = Repository::temporary();
}

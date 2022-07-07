use ::{git2::Repository, save::git2::RepositoryExt};

#[test]
fn cli() {
    let _repo = Repository::temporary();
}

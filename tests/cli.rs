use ::{
    git2::Repository,
    save::{
        git2::RepositoryExt,
        testing::{assert_at, assert_debug_eq},
    },
};

#[test]
fn cli() {
    let mut repo = Repository::temporary();
}

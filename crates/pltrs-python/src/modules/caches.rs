//! `_native.caches`: the hit path of the package's caches keyed by a class
//! held weakly (`_program_classes.class_cache`). The cache is a dict keyed
//! by `(id(cls), *args)`; `get1` and `get2` look a key up and return its
//! value, or call the cache's `miss` function (Python: it computes, stores
//! and has the entries of the class forgotten when the class goes). Bound
//! in a `functools.partial`, a hit runs no Python code.

use crate::binding::prelude::*;
use crate::binding::{IntoPy, tuple};

/// The value of the key `(id(cls), *args)` in `entries`, else what
/// `miss(cls, *args)` gives.
fn lookup(entries: &Owned, miss: &Owned, cls: &Owned, args: &[&Owned]) -> PyResult<Owned> {
    Python::with_gil(|py| {
        let (entries, miss, cls) = (entries.bind(py), miss.bind(py), cls.bind(py));
        let mut key = vec![cls.id().into_py(py)?];
        key.extend(args.iter().map(|arg| arg.bind(py)));
        // `miss` stands for no entry: no cached value is the function.
        let found = entries.call_method("get", &[tuple(py, key)?, miss.clone()])?;
        if !found.is(&miss) {
            return Ok(found.unbind());
        }
        let mut call = vec![cls];
        call.extend(args.iter().map(|arg| arg.bind(py)));
        Ok(miss.call(&call, &[])?.unbind())
    })
}

/// The cached value of a function of a class alone.
fn get1(entries: Owned, miss: Owned, cls: Owned) -> PyResult<Owned> {
    lookup(&entries, &miss, &cls, &[])
}

/// The cached value of a function of a class and one more argument.
fn get2(entries: Owned, miss: Owned, cls: Owned, arg: Owned) -> PyResult<Owned> {
    lookup(&entries, &miss, &cls, &[&arg])
}

pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get1", get1)?;
    module.add("get2", get2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};

    #[test]
    fn hits_answer_from_the_dict_and_misses_from_the_function() {
        with_python(|py| {
            let entries = eval(py, "", "{}").unbind();
            let miss = eval(
                py,
                "",
                "lambda cls, name, calls=[]: (calls.append(name), f'{cls.__name__}.{name}')[1]",
            )
            .unbind();
            let cls = eval(py, "", "int").unbind();
            let name = eval(py, "", "'real'").unbind();
            let first = get2(entries.clone(), miss.clone(), cls.clone(), name.clone()).unwrap();
            assert_eq!(first.bind(py).extract::<String>().unwrap(), "int.real");
            let key = tuple(
                py,
                vec![cls.bind(py).id().into_py(py).unwrap(), name.bind(py)],
            )
            .unwrap();
            entries.bind(py).set_item(key, "cached").unwrap();
            let hit = get2(entries, miss, cls, name).unwrap();
            assert_eq!(hit.bind(py).extract::<String>().unwrap(), "cached");
        });
    }
}

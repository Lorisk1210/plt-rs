//! Steps acting on the artists earlier steps returned.

use plt::artist::ArtistHandle;

use super::axes_images;
use super::call::Call;
use super::scene::{Handled, Object, Produced};

/// Runs a method of `object`.
pub(super) fn call(object: &Object, call: &Call<'_>) -> Handled {
    if let Some(changed) = axes_images::mappable_call(object, call) {
        return Some(changed);
    }
    match (object, call.method) {
        (Object::Collection(collection), "set_offsets") => {
            Some(call.points(0, "offsets").map(|offsets| {
                collection.set_offsets(offsets);
                Produced::Nothing
            }))
        }
        (Object::Collection(collection), _) => generic(collection, call),
        (Object::Image(image), _) => generic(image, call),
        (Object::Artist(artist), _) => generic(artist, call),
        _ => None,
    }
}

/// The methods every artist has.
fn generic(artist: &impl ArtistHandle, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "set_visible" => call.opt_bool(0, "b").and_then(|visible| {
            artist.set_visible(visible.ok_or_else(|| call.bad("missing visibility"))?);
            Ok(Produced::Nothing)
        }),
        "get_visible" => Ok(Produced::Value(artist.visible().into())),
        "remove" => {
            artist.remove();
            Ok(Produced::Nothing)
        }
        _ => return None,
    })
}

"""The exception image comparison tests raise."""

from .._native import register


class ImageComparisonFailure(AssertionError):
    """
    Raise this exception to mark a test as a comparison between two images.
    """


register("ImageComparisonFailure", ImageComparisonFailure)

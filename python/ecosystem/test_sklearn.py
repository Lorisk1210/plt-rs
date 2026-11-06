"""scikit-learn's displays (``sklearn.metrics`` and ``sklearn.inspection``)
on pltrs as matplotlib."""

import numpy as np
import pytest

pytest.importorskip("sklearn")


@pytest.fixture
def labels():
    """True and predicted labels of three classes, and scores."""
    truth = np.arange(60) % 3
    predicted = (truth + (np.arange(60) % 7 == 0)) % 3
    return truth, predicted


@pytest.fixture
def binary():
    """Binary labels and the scores of a classifier for them."""
    truth = (np.arange(80) % 2).astype(int)
    scores = truth * 0.6 + 0.4 * np.abs(np.sin(np.arange(80.0)))
    return truth, scores


def test_confusion_matrix(check, labels):
    from sklearn.metrics import ConfusionMatrixDisplay
    check(ConfusionMatrixDisplay.from_predictions(*labels).figure_)


def test_roc_curve(check, binary):
    from sklearn.metrics import RocCurveDisplay
    check(RocCurveDisplay.from_predictions(*binary, name="model").figure_)


def test_precision_recall(check, binary):
    from sklearn.metrics import PrecisionRecallDisplay
    check(PrecisionRecallDisplay.from_predictions(*binary).figure_)


def test_decision_boundary(check):
    from sklearn.inspection import DecisionBoundaryDisplay
    from sklearn.linear_model import LogisticRegression
    X = np.column_stack([np.linspace(-2, 2, 40), np.sin(np.linspace(0, 6, 40))])
    y = (X[:, 0] + X[:, 1] > 0).astype(int)
    model = LogisticRegression().fit(X, y)
    check(DecisionBoundaryDisplay.from_estimator(model, X, grid_resolution=30).figure_)


def test_partial_dependence(check):
    from sklearn.inspection import PartialDependenceDisplay
    from sklearn.linear_model import LinearRegression
    X = np.column_stack([np.linspace(0, 1, 50), np.cos(np.linspace(0, 3, 50))])
    y = 2 * X[:, 0] - X[:, 1]
    model = LinearRegression().fit(X, y)
    check(PartialDependenceDisplay.from_estimator(model, X, [0, 1]).figure_)

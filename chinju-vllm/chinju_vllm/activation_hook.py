"""
Activation Hook for vLLM Hidden State Extraction

This module provides hooks to extract hidden states from vLLM models
during inference for value neuron monitoring (C15).

Usage:
    from chinju_vllm import ActivationHook, HiddenStateExtractor

    extractor = HiddenStateExtractor(model)
    with extractor.capture_layers([12, 16, 20]) as buffer:
        output = model.generate(input_ids)
        hidden_states = buffer.get_all()
"""

from __future__ import annotations

import logging
from collections import deque
from contextlib import contextmanager, AbstractContextManager
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, Generator, List, Optional, Tuple, Union, cast

import numpy as np
import torch
from torch import nn, Tensor

logger = logging.getLogger(__name__)


@dataclass
class LayerActivation:
    """Single layer activation capture."""

    layer_idx: int
    activation: Tensor
    timestamp_ns: int
    token_positions: Optional[List[int]] = None

    def to_numpy(self) -> np.ndarray:
        """Convert activation to numpy array."""
        return self.activation.detach().cpu().numpy()

    @property
    def shape(self) -> Tuple[int, ...]:
        return tuple(self.activation.shape)


@dataclass
class ActivationBuffer:
    """Thread-safe buffer for storing captured activations."""

    max_size: int = 1000
    _buffer: deque[LayerActivation] = field(default_factory=lambda: deque(maxlen=1000))

    def __post_init__(self) -> None:
        self._buffer = deque(maxlen=self.max_size)

    def add(self, activation: LayerActivation) -> None:
        """Add activation to buffer."""
        self._buffer.append(activation)

    def get_all(self) -> List[LayerActivation]:
        """Get all activations in buffer."""
        return list(self._buffer)

    def get_by_layer(self, layer_idx: int) -> List[LayerActivation]:
        """Get activations for specific layer."""
        return [a for a in self._buffer if a.layer_idx == layer_idx]

    def get_latest(self, n: int = 1) -> List[LayerActivation]:
        """Get n most recent activations."""
        return list(self._buffer)[-n:]

    def clear(self) -> None:
        """Clear all activations."""
        self._buffer.clear()

    def __len__(self) -> int:
        return len(self._buffer)


class ActivationHook:
    """
    Hook for capturing activations from specific layers.

    Registers forward hooks on target layers to capture hidden states
    during inference.
    """

    def __init__(
        self,
        buffer: ActivationBuffer,
        layer_idx: int,
        capture_input: bool = False,
        capture_output: bool = True,
        detach: bool = True,
    ):
        self.buffer = buffer
        self.layer_idx = layer_idx
        self.capture_input = capture_input
        self.capture_output = capture_output
        self.detach = detach
        self._handle: Optional[Any] = None

    def __call__(
        self,
        module: nn.Module,
        input: Tuple[Tensor, ...],
        output: Union[Tensor, Tuple[Tensor, ...]],
    ) -> None:
        """Hook callback - captures activation."""
        import time

        if self.capture_output:
            if isinstance(output, tuple):
                activation = output[0]
            else:
                activation = output

            if self.detach:
                activation = activation.detach().clone()

            self.buffer.add(LayerActivation(
                layer_idx=self.layer_idx,
                activation=activation,
                timestamp_ns=time.time_ns(),
            ))

    def register(self, module: nn.Module) -> None:
        """Register hook on module."""
        self._handle = module.register_forward_hook(self)

    def remove(self) -> None:
        """Remove hook from module."""
        if self._handle is not None:
            self._handle.remove()
            self._handle = None


class HiddenStateExtractor:
    """
    High-level interface for extracting hidden states from vLLM models.

    Supports:
    - Selective layer extraction (specify which layers to monitor)
    - Automatic layer discovery for transformer models
    - Context manager for safe hook management

    Example:
        extractor = HiddenStateExtractor(llm.model)

        # Extract from specific layers
        with extractor.capture_layers([12, 16, 20, 24]) as buffer:
            output = llm.generate(prompts)

            # Analyze captured hidden states
            for activation in buffer.get_all():
                print(f"Layer {activation.layer_idx}: {activation.shape}")
    """

    def __init__(
        self,
        model: nn.Module,
        layer_pattern: str = "layers",
    ):
        """
        Initialize extractor.

        Args:
            model: The LLM model (e.g., LlamaForCausalLM)
            layer_pattern: Attribute name containing transformer layers
        """
        self.model = model
        self.layer_pattern = layer_pattern
        self._hooks: List[ActivationHook] = []
        self._layers: Optional[nn.ModuleList] = None

        # Discover layers
        self._discover_layers()

    def _discover_layers(self) -> None:
        """Discover transformer layers in model."""
        # Try common patterns
        patterns = [
            "model.layers",
            "transformer.h",
            "encoder.layer",
            "decoder.layers",
            self.layer_pattern,
        ]

        for pattern in patterns:
            try:
                module = self.model
                for attr in pattern.split("."):
                    module = getattr(module, attr)
                if isinstance(module, nn.ModuleList):
                    self._layers = module
                    logger.info(f"Found {len(module)} layers at '{pattern}'")
                    return
            except AttributeError:
                continue

        logger.warning("Could not auto-discover layers, manual registration required")

    @property
    def num_layers(self) -> int:
        """Get number of discovered layers."""
        return len(self._layers) if self._layers else 0

    def get_layer(self, idx: int) -> Optional[nn.Module]:
        """Get specific layer by index."""
        if self._layers is None:
            return None
        if idx < 0 or idx >= len(self._layers):
            return None
        return self._layers[idx]

    @contextmanager
    def capture_layers(
        self,
        layer_indices: List[int],
        buffer_size: int = 1000,
    ) -> Generator[ActivationBuffer, None, None]:
        """
        Context manager for capturing activations from specified layers.

        Args:
            layer_indices: List of layer indices to capture
            buffer_size: Maximum activations to store

        Yields:
            ActivationBuffer containing captured activations
        """
        buffer = ActivationBuffer(max_size=buffer_size)
        hooks = []

        try:
            # Register hooks
            for idx in layer_indices:
                layer = self.get_layer(idx)
                if layer is None:
                    logger.warning(f"Layer {idx} not found, skipping")
                    continue

                hook = ActivationHook(buffer, idx)
                hook.register(layer)
                hooks.append(hook)
                logger.debug(f"Registered hook on layer {idx}")

            yield buffer

        finally:
            # Always clean up hooks
            for hook in hooks:
                hook.remove()
            logger.debug(f"Removed {len(hooks)} hooks")

    def capture_all_layers(self, buffer_size: int = 1000) -> AbstractContextManager[ActivationBuffer]:
        """
        Context manager for capturing all layers.

        Use with caution - can consume significant memory.
        """
        if self._layers is None:
            raise ValueError("No layers discovered")

        return self.capture_layers(
            list(range(len(self._layers))),
            buffer_size=buffer_size,
        )

    def capture_strategic_layers(
        self,
        early: int = 1,
        middle: int = 2,
        late: int = 1,
        buffer_size: int = 1000,
    ) -> AbstractContextManager[ActivationBuffer]:
        """
        Capture from strategic layer positions.

        Args:
            early: Number of early layers (first 1/3)
            middle: Number of middle layers
            late: Number of late layers (last 1/3)
            buffer_size: Maximum activations to store
        """
        if self._layers is None:
            raise ValueError("No layers discovered")

        n = len(self._layers)
        third = n // 3

        indices: List[int] = []

        # Early layers
        if early > 0:
            step = max(1, third // early)
            indices.extend(range(0, third, step)[:early])

        # Middle layers
        if middle > 0:
            step = max(1, third // middle)
            indices.extend(range(third, 2 * third, step)[:middle])

        # Late layers
        if late > 0:
            step = max(1, third // late)
            indices.extend(range(2 * third, n, step)[:late])

        return self.capture_layers(sorted(set(indices)), buffer_size)


class VLLMActivationMiddleware:
    """
    Middleware for integrating with vLLM's inference pipeline.

    This class wraps the vLLM LLM instance and intercepts inference
    to capture hidden states automatically.
    """

    def __init__(
        self,
        llm: Any,  # vllm.LLM
        target_layers: Optional[List[int]] = None,
        on_activation: Optional[Callable[[List[LayerActivation]], None]] = None,
    ):
        """
        Initialize middleware.

        Args:
            llm: vLLM LLM instance
            target_layers: Layers to monitor (None = auto-select strategic layers)
            on_activation: Callback for each captured activation batch
        """
        self.llm = llm
        self.target_layers = target_layers
        self.on_activation = on_activation

        # Get underlying model
        self._model = self._get_model()
        self._extractor = HiddenStateExtractor(self._model) if self._model else None

    def _get_model(self) -> Optional[nn.Module]:
        """Extract the underlying model from vLLM."""
        try:
            # vLLM 0.4+ structure
            if hasattr(self.llm, "llm_engine"):
                engine = self.llm.llm_engine
                if hasattr(engine, "model_executor"):
                    executor = engine.model_executor
                    if hasattr(executor, "driver_worker"):
                        worker = executor.driver_worker
                        if hasattr(worker, "model_runner"):
                            runner = worker.model_runner
                            if hasattr(runner, "model"):
                                return cast(nn.Module, runner.model)

            logger.warning("Could not access vLLM internal model")
            return None
        except Exception as e:
            logger.error(f"Error accessing vLLM model: {e}")
            return None

    def generate(
        self,
        prompts: Union[str, List[str]],
        **kwargs: Any,
    ) -> Tuple[Any, List[LayerActivation]]:
        """
        Generate with activation capture.

        Returns:
            Tuple of (vLLM outputs, captured activations)
        """
        if self._extractor is None:
            # Fallback: no activation capture
            logger.warning("Extractor not available, generating without capture")
            return self.llm.generate(prompts, **kwargs), []

        # Determine layers to capture
        if self.target_layers:
            layers = self.target_layers
        else:
            # Auto-select strategic layers
            layers = self._select_strategic_layers()

        # Generate with capture
        with self._extractor.capture_layers(layers) as buffer:
            outputs = self.llm.generate(prompts, **kwargs)
            activations = buffer.get_all()

        # Callback if provided
        if self.on_activation and activations:
            self.on_activation(activations)

        return outputs, activations

    def _select_strategic_layers(self) -> List[int]:
        """Select strategic layers for value neuron monitoring."""
        if self._extractor is None or self._extractor.num_layers == 0:
            return []

        n = self._extractor.num_layers

        # Select layers at 1/4, 1/2, 3/4, and final
        return sorted(set([
            n // 4,
            n // 2,
            3 * n // 4,
            n - 1,
        ]))

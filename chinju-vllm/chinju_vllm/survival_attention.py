"""
Survival Attention Module (C17)

Implements the Survival-Weighted Attention Mechanism:

SurvivalAttention(Q, K, V, S) = softmax(QK^T / sqrt(d_k) + alpha * S) * V

Where S_ij = log(N_j) + log(mu_j / mu_c) - delta_j

Based on CHINJU Protocol C17: 存続性重み付け注意機構システム
"""

from __future__ import annotations

import logging
import math
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Tuple, Union, cast

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch import Tensor

logger = logging.getLogger(__name__)


@dataclass
class SurvivalScore:
    """Survival score for a token/position."""

    n_value: float  # Number of options / diversity
    mu_value: float  # Integrity / coherence with knowledge base
    delta_value: float  # Distance from known facts
    score: float  # Combined survival score

    @classmethod
    def compute(
        cls,
        n: float,
        mu: float,
        mu_c: float = 1.0,
        delta: float = 0.0,
    ) -> "SurvivalScore":
        """
        Compute survival score.

        S = log(N) + log(mu / mu_c) - delta

        Args:
            n: Number of valid options (N in S = N × E × Y)
            mu: Integrity score
            mu_c: Reference integrity (current context)
            delta: Distance from known facts
        """
        score = (
            math.log(max(n, 1e-8))
            + math.log(max(mu / mu_c, 1e-8))
            - delta
        )
        return cls(
            n_value=n,
            mu_value=mu,
            delta_value=delta,
            score=score,
        )


class SurvivalScorer(nn.Module):
    """
    Neural network for computing survival scores.

    Takes token embeddings and produces survival scores.
    Trained on fact-checked datasets to identify factual vs hallucinated content.
    """

    def __init__(
        self,
        hidden_dim: int = 768,
        intermediate_dim: int = 256,
        n_heads: int = 4,
        dropout: float = 0.1,
    ):
        """
        Initialize scorer.

        Args:
            hidden_dim: Input embedding dimension
            intermediate_dim: Hidden layer dimension
            n_heads: Number of attention heads for self-attention
            dropout: Dropout rate
        """
        super().__init__()

        self.hidden_dim = hidden_dim
        self.intermediate_dim = intermediate_dim

        # Project embeddings to intermediate
        self.input_proj = nn.Linear(hidden_dim, intermediate_dim)

        # Self-attention for context awareness
        self.self_attn = nn.MultiheadAttention(
            intermediate_dim,
            n_heads,
            dropout=dropout,
            batch_first=True,
        )

        # Output heads for N, mu, delta
        self.n_head = nn.Sequential(
            nn.Linear(intermediate_dim, intermediate_dim // 2),
            nn.ReLU(),
            nn.Linear(intermediate_dim // 2, 1),
            nn.Softplus(),  # N must be positive
        )

        self.mu_head = nn.Sequential(
            nn.Linear(intermediate_dim, intermediate_dim // 2),
            nn.ReLU(),
            nn.Linear(intermediate_dim // 2, 1),
            nn.Sigmoid(),  # mu in [0, 1]
        )

        self.delta_head = nn.Sequential(
            nn.Linear(intermediate_dim, intermediate_dim // 2),
            nn.ReLU(),
            nn.Linear(intermediate_dim // 2, 1),
            nn.ReLU(),  # delta >= 0
        )

        self._initialize_weights()

    def _initialize_weights(self) -> None:
        """Initialize weights for stable training."""
        for module in self.modules():
            if isinstance(module, nn.Linear):
                nn.init.xavier_uniform_(module.weight)
                if module.bias is not None:
                    nn.init.zeros_(module.bias)

    def forward(
        self,
        embeddings: Tensor,
        attention_mask: Optional[Tensor] = None,
    ) -> Tuple[Tensor, Tensor, Tensor]:
        """
        Compute N, mu, delta for each token.

        Args:
            embeddings: Token embeddings [batch, seq_len, hidden_dim]
            attention_mask: Attention mask [batch, seq_len]

        Returns:
            (N, mu, delta) each of shape [batch, seq_len]
        """
        # Project to intermediate
        x = self.input_proj(embeddings)

        # Apply self-attention
        if attention_mask is not None:
            key_padding_mask = ~attention_mask.bool()
        else:
            key_padding_mask = None

        x, _ = self.self_attn(x, x, x, key_padding_mask=key_padding_mask)

        # Compute each component
        n = self.n_head(x).squeeze(-1)  # [batch, seq_len]
        mu = self.mu_head(x).squeeze(-1)  # [batch, seq_len]
        delta = self.delta_head(x).squeeze(-1)  # [batch, seq_len]

        return n, mu, delta

    def compute_survival_scores(
        self,
        embeddings: Tensor,
        attention_mask: Optional[Tensor] = None,
        mu_c: float = 1.0,
    ) -> Tensor:
        """
        Compute survival scores for all tokens.

        S_ij = log(N_j) + log(mu_j / mu_c) - delta_j

        Args:
            embeddings: Token embeddings
            attention_mask: Attention mask
            mu_c: Reference integrity

        Returns:
            Survival scores [batch, seq_len]
        """
        n, mu, delta = self.forward(embeddings, attention_mask)

        # Compute S = log(N) + log(mu / mu_c) - delta
        scores = (
            torch.log(n + 1e-8)
            + torch.log(mu / mu_c + 1e-8)
            - delta
        )

        return scores


class SurvivalAttentionLayer(nn.Module):
    """
    Survival-Weighted Attention Layer.

    Modifies standard attention to incorporate survival scores:

    SurvivalAttention(Q, K, V, S) = softmax(QK^T / sqrt(d_k) + alpha * S) * V

    Can be used to:
    1. Replace standard attention layers
    2. Wrap existing attention layers
    """

    def __init__(
        self,
        hidden_dim: int,
        n_heads: int,
        alpha: float = 0.1,
        alpha_mode: str = "static",  # "static", "learned", "dynamic"
        scorer: Optional[SurvivalScorer] = None,
        dropout: float = 0.1,
    ):
        """
        Initialize layer.

        Args:
            hidden_dim: Model hidden dimension
            n_heads: Number of attention heads
            alpha: Survival score weight (or initial value if learned)
            alpha_mode: How to handle alpha
                - "static": Fixed alpha value
                - "learned": Learnable parameter
                - "dynamic": Computed from input (creative vs factual)
            scorer: SurvivalScorer instance (None = create new)
            dropout: Dropout rate
        """
        super().__init__()

        self.hidden_dim = hidden_dim
        self.n_heads = n_heads
        self.head_dim = hidden_dim // n_heads
        self.alpha_mode = alpha_mode
        self.dropout = nn.Dropout(dropout)

        # Q, K, V projections
        self.q_proj = nn.Linear(hidden_dim, hidden_dim)
        self.k_proj = nn.Linear(hidden_dim, hidden_dim)
        self.v_proj = nn.Linear(hidden_dim, hidden_dim)
        self.o_proj = nn.Linear(hidden_dim, hidden_dim)

        # Alpha handling
        if alpha_mode == "learned":
            self.alpha = nn.Parameter(torch.tensor(alpha))
        elif alpha_mode == "dynamic":
            self.alpha_net = nn.Sequential(
                nn.Linear(hidden_dim, hidden_dim // 4),
                nn.ReLU(),
                nn.Linear(hidden_dim // 4, n_heads),
                nn.Sigmoid(),  # alpha in [0, 1]
            )
            self._base_alpha = alpha
        else:
            self.register_buffer("alpha", torch.tensor(alpha))

        # Survival scorer
        self.scorer = scorer or SurvivalScorer(hidden_dim)

        self._initialize_weights()

    def _initialize_weights(self) -> None:
        """Initialize projection weights."""
        for proj in [self.q_proj, self.k_proj, self.v_proj, self.o_proj]:
            nn.init.xavier_uniform_(proj.weight)
            nn.init.zeros_(proj.bias)

    def _get_alpha(
        self,
        hidden_states: Tensor,
    ) -> Tensor:
        """Get alpha value(s) based on mode."""
        if self.alpha_mode == "static":
            return self.alpha
        elif self.alpha_mode == "learned":
            return self.alpha
        else:  # dynamic
            # Use CLS token or mean pooling
            pooled = hidden_states.mean(dim=1)  # [batch, hidden_dim]
            alpha_per_head: Tensor = self.alpha_net(pooled)  # [batch, n_heads]
            return alpha_per_head * self._base_alpha

    def forward(
        self,
        hidden_states: Tensor,
        attention_mask: Optional[Tensor] = None,
        position_ids: Optional[Tensor] = None,
        output_attentions: bool = False,
    ) -> Tuple[Tensor, Optional[Tensor]]:
        """
        Forward pass with survival-weighted attention.

        Args:
            hidden_states: Input tensor [batch, seq_len, hidden_dim]
            attention_mask: Attention mask [batch, seq_len] or [batch, 1, 1, seq_len]
            position_ids: Position IDs (unused, for compatibility)
            output_attentions: Whether to return attention weights

        Returns:
            (output, attention_weights if requested)
        """
        batch_size, seq_len, _ = hidden_states.shape

        # Project Q, K, V
        query = self.q_proj(hidden_states)
        key = self.k_proj(hidden_states)
        value = self.v_proj(hidden_states)

        # Reshape for multi-head attention
        query = query.view(batch_size, seq_len, self.n_heads, self.head_dim).transpose(1, 2)
        key = key.view(batch_size, seq_len, self.n_heads, self.head_dim).transpose(1, 2)
        value = value.view(batch_size, seq_len, self.n_heads, self.head_dim).transpose(1, 2)

        # Compute attention scores: QK^T / sqrt(d_k)
        attn_scores = torch.matmul(query, key.transpose(-2, -1)) / math.sqrt(self.head_dim)

        # Compute survival scores
        # Convert 4D mask to 2D for scorer if needed
        scorer_mask = attention_mask
        if attention_mask is not None and attention_mask.dim() == 4:
            # [batch, 1, 1, seq_len] -> [batch, seq_len]
            scorer_mask = attention_mask.squeeze(1).squeeze(1)
        survival_scores = self.scorer.compute_survival_scores(
            hidden_states,
            scorer_mask,
        )  # [batch, seq_len]

        # Expand survival scores for attention: [batch, n_heads, seq_len, seq_len]
        # Each query position looks at all keys, so broadcast survival to key dimension
        survival_broadcast = survival_scores.unsqueeze(1).unsqueeze(2)
        survival_broadcast = survival_broadcast.expand(-1, self.n_heads, seq_len, -1)

        # Get alpha
        alpha = self._get_alpha(hidden_states)
        if self.alpha_mode == "dynamic":
            # alpha is [batch, n_heads], expand to [batch, n_heads, 1, 1]
            alpha = alpha.unsqueeze(-1).unsqueeze(-1)

        # Add survival bias: QK^T / sqrt(d_k) + alpha * S
        attn_scores = attn_scores + alpha * survival_broadcast

        # Apply attention mask
        if attention_mask is not None:
            if attention_mask.dim() == 2:
                # [batch, seq_len] -> [batch, 1, 1, seq_len]
                attention_mask = attention_mask[:, None, None, :]
            attn_scores = attn_scores + (1.0 - attention_mask) * -1e9

        # Softmax and dropout
        attn_weights = F.softmax(attn_scores, dim=-1)
        attn_weights = self.dropout(attn_weights)

        # Apply to values
        output = torch.matmul(attn_weights, value)

        # Reshape back
        output = output.transpose(1, 2).contiguous().view(batch_size, seq_len, self.hidden_dim)

        # Final projection
        output = self.o_proj(output)

        if output_attentions:
            return output, attn_weights
        return output, None


class SurvivalAttentionWrapper:
    """
    Wrapper to add survival attention to existing models.

    Usage:
        wrapper = SurvivalAttentionWrapper(model)
        wrapper.patch_layers([12, 16, 20])

        # Use model as normal
        output = model.generate(input_ids)

        # Restore original
        wrapper.restore()
    """

    def __init__(
        self,
        model: nn.Module,
        alpha: float = 0.1,
        scorer: Optional[SurvivalScorer] = None,
    ):
        """
        Initialize wrapper.

        Args:
            model: The LLM model
            alpha: Survival weight
            scorer: Shared scorer (None = create per layer)
        """
        self.model = model
        self.alpha = alpha
        self.scorer = scorer

        self._original_layers: Dict[int, nn.Module] = {}
        self._patched = False

    def patch_layers(
        self,
        layer_indices: List[int],
        layer_path: str = "model.layers",
    ) -> None:
        """
        Patch specified layers with survival attention.

        Args:
            layer_indices: Layer indices to patch
            layer_path: Path to layers in model (e.g., "model.layers")
        """
        if self._patched:
            logger.warning("Already patched, restoring first")
            self.restore()

        # Get layers
        layers: Any = self.model
        for attr in layer_path.split("."):
            layers = getattr(layers, attr)

        for idx in layer_indices:
            if idx >= len(layers):
                logger.warning(f"Layer {idx} out of range, skipping")
                continue

            layer: nn.Module = layers[idx]

            # Find attention module
            attn_module = None
            attn_attr = None
            for name in ["self_attn", "attention", "attn"]:
                if hasattr(layer, name):
                    attn_module = getattr(layer, name)
                    attn_attr = name
                    break

            if attn_module is None:
                logger.warning(f"No attention found in layer {idx}")
                continue

            # Store original
            self._original_layers[idx] = attn_module

            # Get dimensions
            hidden_dim = self._get_hidden_dim(attn_module)
            n_heads = self._get_n_heads(attn_module)

            # Create survival attention
            survival_attn = SurvivalAttentionLayer(
                hidden_dim=hidden_dim,
                n_heads=n_heads,
                alpha=self.alpha,
                scorer=self.scorer,
            )

            # Copy weights if possible
            self._copy_weights(attn_module, survival_attn)

            # Replace (attn_attr is guaranteed to be set at this point)
            assert attn_attr is not None
            setattr(layer, attn_attr, survival_attn)
            logger.info(f"Patched layer {idx} with survival attention")

        self._patched = True

    def _get_hidden_dim(self, attn: nn.Module) -> int:
        """Extract hidden dimension from attention module."""
        for name in ["hidden_size", "embed_dim", "d_model"]:
            if hasattr(attn, name):
                return int(getattr(attn, name))

        # Try from projection
        for name in ["q_proj", "query", "wq"]:
            if hasattr(attn, name):
                proj = getattr(attn, name)
                if hasattr(proj, "out_features"):
                    return int(proj.out_features)

        raise ValueError("Could not determine hidden dimension")

    def _get_n_heads(self, attn: nn.Module) -> int:
        """Extract number of heads from attention module."""
        for name in ["num_heads", "n_head", "num_attention_heads"]:
            if hasattr(attn, name):
                return int(getattr(attn, name))

        raise ValueError("Could not determine number of heads")

    def _copy_weights(
        self,
        source: nn.Module,
        target: SurvivalAttentionLayer,
    ) -> None:
        """Copy attention weights from source to target."""
        try:
            # Common patterns
            for src_name, tgt_name in [
                ("q_proj", "q_proj"),
                ("k_proj", "k_proj"),
                ("v_proj", "v_proj"),
                ("o_proj", "o_proj"),
                ("query", "q_proj"),
                ("key", "k_proj"),
                ("value", "v_proj"),
                ("dense", "o_proj"),
            ]:
                if hasattr(source, src_name) and hasattr(target, tgt_name):
                    src_layer = getattr(source, src_name)
                    tgt_layer = getattr(target, tgt_name)
                    if hasattr(src_layer, "weight"):
                        tgt_layer.weight.data.copy_(src_layer.weight.data)
                        if hasattr(src_layer, "bias") and src_layer.bias is not None:
                            tgt_layer.bias.data.copy_(src_layer.bias.data)
        except Exception as e:
            logger.warning(f"Could not copy weights: {e}")

    def restore(self) -> None:
        """Restore original attention layers."""
        if not self._patched:
            return

        layers: Any = self.model
        for attr in "model.layers".split("."):
            if hasattr(layers, attr):
                layers = getattr(layers, attr)
            else:
                logger.warning(f"Could not find {attr} in model")
                return

        for idx, original in self._original_layers.items():
            layer: nn.Module = layers[idx]
            for name in ["self_attn", "attention", "attn"]:
                if hasattr(layer, name):
                    setattr(layer, name, original)
                    logger.info(f"Restored layer {idx}")
                    break

        self._original_layers.clear()
        self._patched = False

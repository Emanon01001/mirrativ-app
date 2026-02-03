<script lang="ts">
  let { message, onRetry, onDismiss } = $props<{
    message: string;
    onRetry?: () => void;
    onDismiss?: () => void;
  }>();
</script>

<div class="error-banner" role="alert">
  <div class="error-content">
    <span class="error-icon" aria-hidden="true">⚠</span>
    <p class="error-text">{message}</p>
  </div>
  <div class="error-actions">
    {#if onRetry}
      <button
        class="retry-btn"
        onclick={onRetry}
        aria-label="再試行"
      >
        再試行
      </button>
    {/if}
    {#if onDismiss}
      <button
        class="dismiss-btn"
        onclick={onDismiss}
        aria-label="閉じる"
      >
        ×
      </button>
    {/if}
  </div>
</div>

<style>
  .error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 18px;
    border-radius: 16px;
    background: linear-gradient(135deg, #fee2e2 0%, #fecaca 100%);
    border: 1px solid #fca5a5;
    box-shadow: 0 4px 12px rgba(220, 38, 38, 0.15);
    animation: slideIn 0.3s ease-out;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .error-content {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
  }

  .error-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .error-text {
    margin: 0;
    color: #991b1b;
    font-weight: 600;
    line-height: 1.5;
  }

  .error-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .retry-btn,
  .dismiss-btn {
    border: none;
    border-radius: 8px;
    padding: 6px 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.875rem;
  }

  .retry-btn {
    background: #dc2626;
    color: #fff;
  }

  .retry-btn:hover {
    background: #b91c1c;
  }

  .retry-btn:active {
    transform: scale(0.95);
  }

  .dismiss-btn {
    background: transparent;
    color: #991b1b;
    font-size: 1.2rem;
    padding: 4px 8px;
  }

  .dismiss-btn:hover {
    background: rgba(153, 27, 27, 0.1);
  }

  .dismiss-btn:active {
    transform: scale(0.9);
  }

  button:focus-visible {
    outline: 2px solid #dc2626;
    outline-offset: 2px;
  }
</style>

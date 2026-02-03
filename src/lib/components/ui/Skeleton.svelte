<script lang="ts">
  let { variant = 'card', width, height } = $props<{
    variant?: 'card' | 'text' | 'circle' | 'avatar';
    width?: string;
    height?: string;
  }>();
</script>

<div
  class="skeleton {variant}"
  style:width={width}
  style:height={height}
></div>

<style>
  .skeleton {
    background: linear-gradient(
      90deg,
      rgba(11, 27, 30, 0.06) 0%,
      rgba(11, 27, 30, 0.12) 50%,
      rgba(11, 27, 30, 0.06) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s ease-in-out infinite;
    border-radius: 12px;
  }

  .skeleton.card {
    width: 100%;
    height: 280px;
  }

  .skeleton.text {
    width: 100%;
    height: 20px;
    border-radius: 8px;
  }

  .skeleton.circle {
    width: 44px;
    height: 44px;
    border-radius: 50%;
  }

  .skeleton.avatar {
    width: 60px;
    height: 60px;
    border-radius: 50%;
  }

  @keyframes shimmer {
    0% {
      background-position: -200% 0;
    }
    100% {
      background-position: 200% 0;
    }
  }
</style>

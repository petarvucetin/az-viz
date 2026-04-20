<script lang="ts">
  export let data: { label: string };
</script>

<div class="rg">
  <div class="rg-label">{data.label}</div>
</div>

<style>
  .rg {
    width: 100%; height: 100%;
    background: linear-gradient(180deg, #fafcff, #eef5ff);
    border: 1.5px dashed #4a90e2;
    border-radius: 10px;
    position: relative;
    pointer-events: none;
  }
  .rg-label {
    position: absolute;
    top: -12px;
    left: 50%;
    transform: translateX(-50%);
    background: #ffffff;
    padding: 2px 12px;
    color: #4a90e2;
    font-size: 12px;
    font-weight: 700;
    font-family: system-ui, sans-serif;
    white-space: nowrap;
    pointer-events: none;
  }
</style>

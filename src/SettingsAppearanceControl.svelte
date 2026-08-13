<script lang="ts">
  import type { Appearance } from "./appearance";

  const OPTIONS: { value: Appearance; label: string }[] = [
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
    { value: "system", label: "System" },
  ];

  let { value, onSelect }: { value: Appearance; onSelect: (value: Appearance) => void } =
    $props();
</script>

<div class="appearance-control" role="radiogroup" aria-label="Appearance">
  {#each OPTIONS as option}
    <button
      type="button"
      class:active={value === option.value}
      role="radio"
      aria-checked={value === option.value}
      onclick={() => onSelect(option.value)}
    >{option.label}</button>
  {/each}
</div>

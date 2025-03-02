<!--
    Copyright: Ankitects Pty Ltd and contributors
    License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
-->
<script lang="ts">
    import { onDestroy } from "svelte";
    import Tooltip from "bootstrap/js/dist/tooltip";

    type TriggerType =
        | "hover focus"
        | "click"
        | "hover"
        | "focus"
        | "manual"
        | "click hover"
        | "click focus"
        | "click hover focus";

    export let tooltip: string;
    export let trigger: TriggerType = "hover focus";

    export let placement: "auto" | "top" | "bottom" | "left" | "right" = "top";
    export let html = true;
    export let offset: Tooltip.Offset = [0, 0];
    export let showDelay = 0;
    export let hideDelay = 0;

    let tooltipObject: Tooltip;
    function createTooltip(element: HTMLElement): void {
        element.title = tooltip;
        tooltipObject = new Tooltip(element, {
            placement,
            html,
            offset,
            delay: { show: showDelay, hide: hideDelay },
            trigger,
        });
    }

    onDestroy(() => tooltipObject?.dispose());

    // hack to update field description tooltips
    let previousTooltip: string = tooltip;
    $: if (tooltip !== previousTooltip) {
        previousTooltip = tooltip;
        let element: HTMLElement = tooltipObject["_element"];
        tooltipObject.dispose();
        createTooltip(element);
    }
</script>

<slot {createTooltip} {tooltipObject} />

<style lang="scss">
    /* tooltip is inserted under the body tag
    /* long tooltips can cause x-overflow */
    :global(body) {
        overflow-x: hidden;
    }
</style>

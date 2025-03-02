<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
-->
<script context="module" lang="ts">
    import type { EditingAreaAPI } from "./EditingArea.svelte";
    import contextProperty from "../sveltelib/context-property";
    import type { Readable } from "svelte/store";

    export interface FieldData {
        name: string;
        description: string;
        fontFamily: string;
        fontSize: number;
        direction: "ltr" | "rtl";
    }

    export interface EditorFieldAPI {
        element: Promise<HTMLElement>;
        direction: Readable<"ltr" | "rtl">;
        editingArea: EditingAreaAPI;
    }

    const key = Symbol("editorField");
    const [set, getEditorField, hasEditorField] = contextProperty<EditorFieldAPI>(key);

    export { getEditorField, hasEditorField };
</script>

<script lang="ts">
    import EditingArea from "./EditingArea.svelte";
    import LabelContainer from "./LabelContainer.svelte";
    import LabelDescription from "./LabelDescription.svelte";
    import LabelName from "./LabelName.svelte";
    import FieldState from "./FieldState.svelte";

    import { onDestroy, setContext } from "svelte";
    import { writable } from "svelte/store";
    import type { Writable } from "svelte/store";
    import { directionKey } from "../lib/context-keys";
    import { promiseWithResolver } from "../lib/promise";
    import type { Destroyable } from "./destroyable";

    export let content: Writable<string>;
    export let field: FieldData;
    export let autofocus = false;

    export let api: (Partial<EditorFieldAPI> & Destroyable) | undefined = undefined;

    const directionStore = writable<"ltr" | "rtl">();
    setContext(directionKey, directionStore);

    $: $directionStore = field.direction;

    const editingArea: Partial<EditingAreaAPI> = {};
    const [element, elementResolve] = promiseWithResolver<HTMLElement>();

    const editorFieldApi = set({
        element,
        direction: directionStore,
        editingArea: editingArea as EditingAreaAPI,
    });

    if (api) {
        Object.assign(api, editorFieldApi);
    }

    onDestroy(() => api?.destroy());
</script>

<div
    use:elementResolve
    class="editor-field"
    on:focusin
    on:focusout
    on:click={() => editingArea.focus?.()}
>
    <LabelContainer>
        <span>
            <LabelName>
                {field.name}
            </LabelName>
            {#if field.description}
                <LabelDescription description={field.description} />
            {/if}
        </span>
        <FieldState><slot name="field-state" /></FieldState>
    </LabelContainer>
    <EditingArea
        {content}
        {autofocus}
        fontFamily={field.fontFamily}
        fontSize={field.fontSize}
        api={editingArea}
    >
        <slot name="editing-inputs" />
    </EditingArea>
</div>

<style lang="scss">
    .editor-field {
        --border-color: var(--border);

        border-radius: 5px;
        border: 1px solid var(--border-color);

        min-width: 0;

        &:focus-within {
            --border-color: var(--focus-border);

            outline: none;
            box-shadow: 0 0 0 3px var(--focus-shadow);
        }
    }
</style>

# Onboarding Flow Test Plan

**Project:** Runie  
**Component:** Onboarding UI Flow (`runie-tui/src/components/onboarding/`)  
**Date:** 2026-05-24  
**Status:** Comprehensive Test Specification

---

## Overview

The onboarding flow guides users through configuring their AI provider credentials. It consists of 5 steps: Welcome → ProviderSelect → KeyInput → ModelSelect → Complete.

### Step Definitions

| Step | Purpose | Valid Navigation |
|------|---------|------------------|
| `Welcome` | Entry point | Next → ProviderSelect |
| `ProviderSelect` | Choose AI provider (21 providers) | Back → Welcome, Next → KeyInput |
| `KeyInput` | Enter API key with masking | Back → ProviderSelect, Next → ModelSelect |
| `ModelSelect` | Choose model for selected provider | Back → KeyInput, Next → Complete |
| `Complete` | Confirm setup, add more providers | Back → ModelSelect |

### Key Data Structures

```rust
struct Onboarding {
    step: OnboardingStep,
    selected_item: usize,          // cursor position in filtered list
    selected_provider: Option<usize>,
    api_key_input: String,
    selected_model: Option<usize>,
    providers: Vec<ProviderOption>,
    models: Vec<ModelOption>,
    error_message: Option<String>,
    search_query: String,
    filtered_provider_indices: Vec<usize>,
    filtered_model_indices: Vec<usize>,
    is_fetching_models: bool,
}

struct ProviderOption {
    name: String,
    id: String,
    description: String,
    key_prefix: String,  // "sk-" for OpenAI, "sk-ant-" for Anthropic, "" for others
}

struct ModelOption {
    name: String,
    id: String,
    description: String,
}
```

---

## 1. Step Transitions

### TC-1.1: Happy Path Forward Navigation

| Field | Value |
|-------|-------|
| **Test Name** | `test_step_transitions_happy_path` |
| **Preconditions** | Fresh Onboarding instance, step = Welcome |
| **Actions** | 1. Call `next_step()` → ProviderSelect<br>2. Call `update_search("")` to populate filtered list<br>3. Call `select_provider(15)` (OpenAI at sorted index 15)<br>4. Call `next_step()` → KeyInput<br>5. Set `api_key_input = "sk-test123"`<br>6. Call `next_step()` → ModelSelect<br>7. Call `update_search("")` to populate filtered list<br>8. Call `select_model(0)`<br>9. Call `next_step()` → Complete |
| **Expected Results** | Step progresses: Welcome → ProviderSelect → KeyInput → ModelSelect → Complete<br>Provider selected: OpenAI (index 15)<br>Model selected: index 0<br>API key stored: "sk-test123" |

### TC-1.2: Back Navigation Through All Steps

| Field | Value |
|-------|-------|
| **Test Name** | `test_back_navigation_full` |
| **Preconditions** | step = Complete |
| **Actions** | 1. Call `prev_step()` → ModelSelect<br>2. Call `prev_step()` → KeyInput<br>3. Call `prev_step()` → ProviderSelect<br>4. Call `prev_step()` → Welcome<br>5. Call `prev_step()` (should stay at Welcome) |
| **Expected Results** | Step regresses: Complete → ModelSelect → KeyInput → ProviderSelect → Welcome → Welcome |

### TC-1.3: Cannot Advance Without Provider Selection

| Field | Value |
|-------|-------|
| **Test Name** | `test_cannot_advance_no_provider` |
| **Preconditions** | step = ProviderSelect, no provider selected |
| **Actions** | 1. Call `update_search("")` to populate filter<br>2. Call `next_step()` without selecting provider |
| **Expected Results** | Step stays at ProviderSelect<br>`error_message` = Some("Please select a provider") |

### TC-1.4: Cannot Advance Without Valid API Key

| Field | Value |
|-------|-------|
| **Test Name** | `test_cannot_advance_invalid_key` |
| **Preconditions** | step = KeyInput, provider = OpenAI, api_key_input = "" |
| **Actions** | 1. Call `next_step()` with empty key |
| **Expected Results** | Step stays at KeyInput<br>`error_message` = Some("Invalid API key format") |

### TC-1.5: Cannot Advance Without Model Selection

| Field | Value |
|-------|-------|
| **Test Name** | `test_cannot_advance_no_model` |
| **Preconditions** | step = ModelSelect, no model selected |
| **Actions** | 1. Call `update_search("")` to populate filter<br>2. Call `next_step()` without selecting model |
| **Expected Results** | Step stays at ModelSelect<br>`error_message` = Some("Please select a model") |

---

## 2. Navigation & Input

### TC-2.1: Navigate Up at First Item

| Field | Value |
|-------|-------|
| **Test Name** | `test_navigate_up_at_boundary` |
| **Preconditions** | step = ProviderSelect, selected_item = 0 |
| **Actions** | 1. Call `update_search("")`<br>2. Call `navigate_up()` repeatedly |
| **Expected Results** | `selected_item` remains 0 (boundary clamped) |

### TC-2.2: Navigate Down at Last Item

| Field | Value |
|-------|-------|
| **Test Name** | `test_navigate_down_at_boundary` |
| **Preconditions** | step = ProviderSelect, selected_item at max index |
| **Actions** | 1. Call `update_search("")`<br>2. Navigate down to last index (20 for 21 providers)<br>3. Call `navigate_down()` again |
| **Expected Results** | `selected_item` stays at max (20), does not exceed bounds |

### TC-2.3: Navigation Bounds with Filtered List

| Field | Value |
|-------|-------|
| **Test Name** | `test_navigation_bounds_filtered` |
| **Preconditions** | step = ProviderSelect, search = "ope" (filters to 2 providers) |
| **Actions** | 1. Call `update_search("ope")`<br>2. Verify filtered count = 2<br>3. Navigate up from index 0<br>4. Navigate down to index 1<br>5. Navigate down from index 1 |
| **Expected Results** | `selected_item` clamped to [0, 1] range<br>Cannot navigate below 0 or above 1 |

### TC-2.4: API Key Character Input

| Field | Value |
|-------|-------|
| **Test Name** | `test_api_key_input` |
| **Preconditions** | step = KeyInput, api_key_input = "" |
| **Actions** | 1. Append characters: "s", "k", "-", "t", "e", "s", "t"<br>2. Verify key length grows |
| **Expected Results** | `api_key_input` = "sk-test"<br>Key masking shows last character visible |

### TC-2.5: API Key Backspace

| Field | Value |
|-------|-------|
| **Test Name** | `test_api_key_backspace` |
| **Preconditions** | step = KeyInput, api_key_input = "sk-test" |
| **Actions** | 1. Remove last character repeatedly |
| **Expected Results** | After 5 backspaces: "sk-"<br>After 6th: "sk"<br>After 7th: "s"<br>After 8th: "" |

### TC-2.6: Search Query Case Insensitivity

| Field | Value |
|-------|-------|
| **Test Name** | `test_search_case_insensitive` |
| **Preconditions** | step = ProviderSelect |
| **Actions** | 1. Call `update_search("OPE")`<br>2. Call `update_search("ope")`<br>3. Call `update_search("OpE")` |
| **Expected Results** | All searches produce same result (lowercased internally)<br>`search_query` stored as lowercase<br>`filtered_provider_indices` contains OpenAI + OpenRouter |

---

## 3. Provider Selection

### TC-3.1: Select OpenAI Provider

| Field | Value |
|-------|-------|
| **Test Name** | `test_select_openai_provider` |
| **Preconditions** | step = ProviderSelect, search populated |
| **Actions** | 1. Call `update_search("")`<br>2. Call `select_provider(15)` (OpenAI at sorted index) |
| **Expected Results** | `selected_provider` = Some(15)<br>`models` populated with 3 OpenAI models: gpt-4o, gpt-4o-mini, o1-mini |

### TC-3.2: Select Anthropic Provider

| Field | Value |
|-------|-------|
| **Test Name** | `test_select_anthropic_provider` |
| **Preconditions** | step = ProviderSelect |
| **Actions** | 1. Call `update_search("")`<br>2. Call `select_provider(0)` (Anthropic at sorted index 0) |
| **Expected Results** | `selected_provider` = Some(0)<br>`models` populated with 3 Anthropic models: claude-haiku, claude-opus, claude-sonnet-4 |

### TC-3.3: Model Selection Clears on Provider Change

| Field | Value |
|-------|-------|
| **Test Name** | `test_model_clears_on_provider_change` |
| **Preconditions** | Provider = OpenAI, model = GPT-4o Mini (index 1) selected |
| **Actions** | 1. Change provider to Anthropic |
| **Expected Results** | `selected_model` = None<br>Model selection reset when provider changes |

### TC-3.4: Provider Models Alphabetically Sorted

| Field | Value |
|-------|-------|
| **Test Name** | `test_openai_models_sorted` |
| **Preconditions** | OpenAI provider selected |
| **Actions** | 1. Select OpenAI provider<br>2. Inspect `models` order |
| **Expected Results** | Models sorted: GPT-4o < GPT-4o Mini < O1 Mini<br>`models[0].id` = "gpt-4o"<br>`models[1].id` = "gpt-4o-mini"<br>`models[2].id` = "o1-mini" |

### TC-3.5: All 21 Providers Available

| Field | Value |
|-------|-------|
| **Test Name** | `test_all_providers_present` |
| **Preconditions** | Fresh Onboarding instance |
| **Actions** | 1. Inspect `providers` list |
| **Expected Results** | `providers.len()` = 21<br>Contains: OpenAI, Anthropic, Google, Cohere, Mistral, DeepSeek, Groq, OpenRouter, HuggingFace, xAI, Azure, Moonshot, Perplexity, Ollama, Hyperbolic, Together, ZAI, MiniMax, Mira, Galadriel, Llamafile |

---

## 4. API Key Handling

### TC-4.1: Valid OpenAI Key Accepted

| Field | Value |
|-------|-------|
| **Test Name** | `test_validate_openai_key_valid` |
| **Preconditions** | Provider = OpenAI (key_prefix = "sk-") |
| **Actions** | 1. Set `api_key_input = "sk-abc123"`<br>2. Call `validate_key()` |
| **Expected Results** | `validate_key()` returns true |

### TC-4.2: OpenAI Key Wrong Prefix Rejected

| Field | Value |
|-------|-------|
| **Test Name** | `test_validate_openai_key_wrong_prefix` |
| **Preconditions** | Provider = OpenAI |
| **Actions** | 1. Set `api_key_input = "pk-abc123"`<br>2. Call `validate_key()` |
| **Expected Results** | `validate_key()` returns false |

### TC-4.3: Valid Anthropic Key Accepted

| Field | Value |
|-------|-------|
| **Test Name** | `test_validate_anthropic_key_valid` |
| **Preconditions** | Provider = Anthropic (key_prefix = "sk-ant-") |
| **Actions** | 1. Set `api_key_input = "sk-ant-api03-..."`<br>2. Call `validate_key()` |
| **Expected Results** | `validate_key()` returns true |

### TC-4.4: Anthropic Key Wrong Prefix Rejected

| Field | Value |
|-------|-------|
| **Test Name** | `test_validate_anthropic_key_wrong_prefix` |
| **Preconditions** | Provider = Anthropic |
| **Actions** | 1. Set `api_key_input = "sk-wrong"`<br>2. Call `validate_key()` |
| **Expected Results** | `validate_key()` returns false |

### TC-4.5: Google Accepts Any Non-Empty Key

| Field | Value |
|-------|-------|
| **Test Name** | `test_validate_google_any_key` |
| **Preconditions** | Provider = Google (key_prefix = "") |
| **Actions** | 1. Set `api_key_input = "AIzaSy..."`<br>2. Set `api_key_input = "any-format"`<br>3. Set `api_key_input = "not-google"`<br>4. Call `validate_key()` for each |
| **Expected Results** | All return true (empty prefix = accept any non-empty) |

### TC-4.6: Empty Key Rejected for All Providers

| Field | Value |
|-------|-------|
| **Test Name** | `test_validate_key_empty_rejected` |
| **Preconditions** | Any provider selected |
| **Actions** | 1. Set `api_key_input = ""`<br>2. Call `validate_key()` |
| **Expected Results** | Returns false |

### TC-4.7: Whitespace-Only Key Rejected

| Field | Value |
|-------|-------|
| **Test Name** | `test_validate_key_whitespace_rejected` |
| **Preconditions** | Provider = OpenAI |
| **Actions** | 1. Set `api_key_input = "   "`<br>2. Call `validate_key()` |
| **Expected Results** | Returns false (trimmed empty check) |

### TC-4.8: Key Masking Display

| Field | Value |
|-------|-------|
| **Test Name** | `test_key_masking_display` |
| **Preconditions** | step = KeyInput |
| **Actions** | 1. Set `api_key_input = "sk-test123"`<br>2. Render and inspect masked output |
| **Expected Results** | Shows "sk-●●●3" (last char visible, middle masked with ●) |

---

## 5. Model Fetching & Selection

### TC-5.1: Model Fetch Triggered on Valid Key Entry

| Field | Value |
|-------|-------|
| **Test Name** | `test_model_fetch_triggered` |
| **Preconditions** | step = KeyInput, provider = OpenAI |
| **Actions** | 1. Set `api_key_input = "sk-valid"`<br>2. Verify `validate_key()` = true |
| **Expected Results** | UI should show "loading models..." indicator<br>`is_fetching_models` should be set during fetch |

### TC-5.2: Loading State Displayed During Fetch

| Field | Value |
|-------|-------|
| **Test Name** | `test_loading_state_during_fetch` |
| **Preconditions** | step = KeyInput, key entered |
| **Actions** | 1. Enter valid key<br>2. Observe render output during fetch |
| **Expected Results** | Render shows "[✓] valid" after successful fetch<br>Shows "loading models..." during fetch |

### TC-5.3: Models Replaced When Fetch Completes

| Field | Value |
|-------|-------|
| **Test Name** | `test_models_replaced_on_fetch_complete` |
| **Preconditions** | step = KeyInput, hardcoded models loaded |
| **Actions** | 1. Enter valid key, fetch initiated<br>2. Simulate successful fetch completing |
| **Expected Results** | `models` updated with fetched models<br>Previous hardcoded models replaced |

### TC-5.4: Fallback to Hardcoded Models on Fetch Failure

| Field | Value |
|-------|-------|
| **Test Name** | `test_fallback_hardcoded_models` |
| **Preconditions** | step = KeyInput |
| **Actions** | 1. Enter valid key<br>2. Simulate fetch failure |
| **Expected Results** | `models` contains hardcoded provider models<br>User can still select from fallback list |

### TC-5.5: Provider Change Resets Model

| Field | Value |
|-------|-------|
| **Test Name** | `test_provider_change_resets_model` |
| **Preconditions** | Provider A selected, model selected |
| **Actions** | 1. Select Provider B |
| **Expected Results** | `selected_model` = None<br>`models` repopulated for Provider B |

### TC-5.6: Select Model from Filtered List

| Field | Value |
|-------|-------|
| **Test Name** | `test_select_model_from_filtered` |
| **Preconditions** | step = ModelSelect, search = "mini" (filters to 2 models) |
| **Actions** | 1. Call `update_search("mini")`<br>2. Call `select_model(0)` |
| **Expected Results** | `selected_model` = Some(real_index_of_first_filtered_model) |

---

## 6. Settings Persistence

### TC-6.1: to_settings Returns None When Incomplete

| Field | Value |
|-------|-------|
| **Test Name** | `test_to_settings_incomplete` |
| **Preconditions** | step = Welcome (no selections made) |
| **Actions** | 1. Call `to_settings()` |
| **Expected Results** | Returns None |

### TC-6.2: to_settings Returns Settings When Complete

| Field | Value |
|-------|-------|
| **Test Name** | `test_to_settings_complete` |
| **Preconditions** | step = Complete, provider = OpenAI, model = GPT-4o, key = "sk-test" |
| **Actions** | 1. Configure all fields<br>2. Call `to_settings()` |
| **Expected Results** | Returns Some(Settings) with correct fields:<br>provider_id = "openai"<br>model_id = "gpt-4o"<br>api_key = "sk-test" |

### TC-6.3: is_complete Returns False When Incomplete

| Field | Value |
|-------|-------|
| **Test Name** | `test_is_complete_false` |
| **Preconditions** | step = ProviderSelect, no provider selected |
| **Actions** | 1. Call `is_complete()` |
| **Expected Results** | Returns false |

### TC-6.4: is_complete Returns True Only When All Fields Set

| Field | Value |
|-------|-------|
| **Test Name** | `test_is_complete_true` |
| **Preconditions** | step = Complete with all fields set |
| **Actions** | 1. Configure provider, model, api_key<br>2. Call `is_complete()` after each field |
| **Expected Results** | Returns false until all 3 fields + step=Complete set<br>Returns true only when complete |

---

## 7. Error Handling

### TC-7.1: Error Message on Missing Provider

| Field | Value |
|-------|-------|
| **Test Name** | `test_error_no_provider_selected` |
| **Preconditions** | step = ProviderSelect, no selection |
| **Actions** | 1. Call `next_step()` |
| **Expected Results** | `error_message` = Some("Please select a provider") |

### TC-7.2: Error Message on Invalid Key

| Field | Value |
|-------|-------|
| **Test Name** | `test_error_invalid_key_format` |
| **Preconditions** | step = KeyInput, key = "bad-key" |
| **Actions** | 1. Call `next_step()` |
| **Expected Results** | `error_message` = Some("Invalid API key format") |

### TC-7.3: Error Message on Missing Model

| Field | Value |
|-------|-------|
| **Test Name** | `test_error_no_model_selected` |
| **Preconditions** | step = ModelSelect, no model selected |
| **Actions** | 1. Call `next_step()` |
| **Expected Results** | `error_message` = Some("Please select a model") |

### TC-7.4: Error Message Cleared on Valid Input

| Field | Value |
|-------|-------|
| **Test Name** | `test_error_cleared_on_valid_input` |
| **Preconditions** | step = KeyInput, error_message set, key invalid |
| **Actions** | 1. Set valid key format<br>2. Call `next_step()` |
| **Expected Results** | `error_message` = None<br>Successfully transitions to ModelSelect |

### TC-7.5: Error State Survives Navigation

| Field | Value |
|-------|-------|
| **Test Name** | `test_error_survives_navigation` |
| **Preconditions** | step = ProviderSelect, error_message set |
| **Actions** | 1. Navigate up/down (change selected_item)<br>2. Call `prev_step()` then `next_step()` |
| **Expected Results** | Error message persists until valid selection made |

---

## 8. Multi-Provider Flow

### TC-8.1: Complete Step Has Yes/No Options

| Field | Value |
|-------|-------|
| **Test Name** | `test_complete_has_add_provider_options` |
| **Preconditions** | step = Complete |
| **Actions** | 1. Inspect render output |
| **Expected Results** | Dialog shows "add another provider?"<br>Options: "yes" (selected by default), "no, finish" |

### TC-8.2: Select Yes Returns to ProviderSelect

| Field | Value |
|-------|-------|
| **Test Name** | `test_select_yes_restarts_provider_select` |
| **Preconditions** | step = Complete, selected_item = 0 (yes) |
| **Actions** | 1. Call `next_step()` |
| **Expected Results** | Step transitions to ProviderSelect<br>Settings preserved for later save |

### TC-8.3: Select No Finishes Onboarding

| Field | Value |
|-------|-------|
| **Test Name** | `test_select_no_finishes` |
| **Preconditions** | step = Complete, selected_item = 1 (no) |
| **Actions** | 1. Navigate down (selected_item = 1)<br>2. Call `next_step()` |
| **Expected Results** | `onboarding` set to None (onboarding complete) |

### TC-8.4: Navigate Between Yes/No Options

| Field | Value |
|-------|-------|
| **Test Name** | `test_navigate_yes_no` |
| **Preconditions** | step = Complete |
| **Actions** | 1. Verify `selected_item` = 0 (yes)<br>2. Call `navigate_down()`<br>3. Call `navigate_down()`<br>4. Call `navigate_up()` |
| **Expected Results** | `selected_item` cycles: 0 → 1 → 1 (max clamped) → 0 |

### TC-8.5: Complete Step Shows Configured Provider

| Field | Value |
|-------|-------|
| **Test Name** | `test_complete_shows_configured` |
| **Preconditions** | step = Complete, provider = OpenAI |
| **Actions** | 1. Render complete step |
| **Expected Results** | Shows "openai configured   1 model" |

---

## 9. Edge Cases

### TC-9.1: Empty Filter Shows All Providers

| Field | Value |
|-------|-------|
| **Test Name** | `test_empty_filter_shows_all` |
| **Preconditions** | step = ProviderSelect |
| **Actions** | 1. Call `update_search("")`<br>2. Inspect `filtered_provider_indices` |
| **Expected Results** | `filtered_provider_indices` empty triggers fallback to all<br>`get_filtered_provider_count()` = 21 |

### TC-9.2: No Filter Matches Shows Empty List

| Field | Value |
|-------|-------|
| **Test Name** | `test_no_filter_matches_empty` |
| **Preconditions** | step = ProviderSelect |
| **Actions** | 1. Call `update_search("xyz")` |
| **Expected Results** | `filtered_provider_indices` empty<br>`get_filtered_provider_count()` = 0<br>User cannot select (no items) |

### TC-9.3: Search Clears on Step Entry

| Field | Value |
|-------|-------|
| **Test Name** | `test_search_clears_on_step_change` |
| **Preconditions** | step = ProviderSelect, search = "ope" |
| **Actions** | 1. Call `prev_step()` to Welcome<br>2. Call `next_step()` back to ProviderSelect |
| **Expected Results** | `search_query` and filters may be restored via `enter_step()`<br>Selection clamped to valid range |

### TC-9.4: Selection Clamped on Filter Reduction

| Field | Value |
|-------|-------|
| **Test Name** | `test_selection_clamped_on_filter` |
| **Preconditions** | step = ProviderSelect, selected_item = 10, then filter to 2 items |
| **Actions** | 1. Select item 10<br>2. Apply filter "xyz" (0 results) or "ope" (2 results) |
| **Expected Results** | `selected_item` clamped to `min(10, filtered_count - 1)` |

### TC-9.5: Clear Search Restores Full List

| Field | Value |
|-------|-------|
| **Test Name** | `test_clear_search_restores_list` |
| **Preconditions** | step = ProviderSelect, search = "ope" |
| **Actions** | 1. Call `clear_search()` |
| **Expected Results** | `search_query` = ""<br>`filtered_provider_indices` cleared<br>`get_filtered_provider_count()` = 21 |

### TC-9.6: Enter Step Restores Filter

| Field | Value |
|-------|-------|
| **Test Name** | `test_enter_step_restores_filter` |
| **Preconditions** | step = ProviderSelect, search = "ope", navigate away and back |
| **Actions** | 1. Set search "ope"<br>2. Call `prev_step()` (Welcome)<br>3. Call `next_step()` (back to ProviderSelect) |
| **Expected Results** | `enter_step()` restores `filtered_provider_indices` based on `search_query` |

### TC-9.7: Provider Without Key Prefix

| Field | Value |
|-------|-------|
| **Test Name** | `test_provider_no_key_prefix` |
| **Preconditions** | Provider = Ollama (key_prefix = "") |
| **Actions** | 1. Select Ollama<br>2. Enter any non-empty key<br>3. Validate |
| **Expected Results** | `validate_key()` returns true for any non-empty key |

### TC-9.8: Model Search Preserves Selection

| Field | Value |
|-------|-------|
| **Test Name** | `test_model_search_preserves_selection` |
| **Preconditions** | step = ModelSelect, model = index 2 selected |
| **Actions** | 1. Apply search "mini" (filters list)<br>2. Clear search |
| **Expected Results** | `selected_model` index preserved (points to real index in full list) |

### TC-9.9: Fuzzy Match Provider Search

| Field | Value |
|-------|-------|
| **Test Name** | `test_fuzzy_match_providers` |
| **Preconditions** | step = ProviderSelect |
| **Actions** | 1. Search "ope"<br>2. Search "goo"<br>3. Search "ant" |
| **Expected Results** | "ope" → OpenAI + OpenRouter<br>"goo" → Google<br>"ant" → Anthropic |

### TC-9.10: Fuzzy Match Model Search

| Field | Value |
|-------|-------|
| **Test Name** | `test_fuzzy_match_models` |
| **Preconditions** | Provider = OpenAI |
| **Actions** | 1. Search "gpt"<br>2. Search "4o"<br>3. Search "mini" |
| **Expected Results** | "gpt" → GPT-4o + GPT-4o Mini<br>"4o" → GPT-4o + GPT-4o Mini<br>"mini" → GPT-4o Mini + O1 Mini |

---

## 10. Async Behavior Tests

### TC-10.1: Model Fetch Starts on Valid Key

| Field | Value |
|-------|-------|
| **Test Name** | `test_fetch_starts_on_valid_key` |
| **Preconditions** | step = KeyInput, key valid |
| **Actions** | 1. Enter valid key<br>2. Wait for fetch to initiate |
| **Expected Results** | `is_fetching_models` set to true<br>Fetch request dispatched |

### TC-10.2: Loading State Shown During Fetch

| Field | Value |
|-------|-------|
| **Test Name** | `test_loading_state_shown` |
| **Preconditions** | Fetch in progress |
| **Actions** | 1. Render KeyInput step |
| **Expected Results** | Shows "loading models..." instead of validation indicator |

### TC-10.3: Models Replaced After Fetch

| Field | Value |
|-------|-------|
| **Test Name** | `test_models_replaced_after_fetch` |
| **Preconditions** | step = ModelSelect, hardcoded models loaded |
| **Actions** | 1. Simulate successful fetch completing<br>2. Inspect models list |
| **Expected Results** | `models` updated with fetched list<br>Old hardcoded list replaced |

### TC-10.4: Fallback on Fetch Failure

| Field | Value |
|-------|-------|
| **Test Name** | `test_fallback_on_fetch_failure` |
| **Preconditions** | step = KeyInput or ModelSelect |
| **Actions** | 1. Enter valid key, initiate fetch<br>2. Simulate network failure |
| **Expected Results** | `models` retains hardcoded fallback<br>User can continue with limited model set |

### TC-10.5: Fetch Error Does Not Block Progression

| Field | Value |
|-------|-------|
| **Test Name** | `test_fetch_error_no_block` |
| **Preconditions** | step = KeyInput, fetch failed |
| **Actions** | 1. With fallback models available, navigate to ModelSelect |
| **Expected Results** | Can still select model and advance<br>Onboarding not stuck |

### TC-10.6: Multiple Rapid Key Changes

| Field | Value |
|-------|-------|
| **Test Name** | `test_rapid_key_changes` |
| **Preconditions** | step = KeyInput |
| **Actions** | 1. Enter "sk-test1"<br>2. Clear, enter "sk-test2"<br>3. Clear, enter "sk-test3" |
| **Expected Results** | Only final key stored<br>No race conditions in validation |

### TC-10.7: Fetch Cancelled on Provider Change

| Field | Value |
|-------|-------|
| **Test Name** | `test_fetch_cancelled_on_provider_change` |
| **Preconditions** | Fetch in progress for Provider A |
| **Actions** | 1. Change selected provider to B |
| **Expected Results** | Fetch for A cancelled/ignored<br>Models populated for B |

---

## Key Flows Summary

### Happy Path Flow

```
Welcome → [next] → ProviderSelect → [select OpenAI] → KeyInput
→ [enter sk-test123] → ModelSelect → [select GPT-4o] → Complete
→ [select "no"] → Done
```

### Filter Providers Flow

```
ProviderSelect → [type "ope"] → [OpenAI + OpenRouter visible]
→ [select OpenAI] → KeyInput
```

### Filter Models Flow

```
ModelSelect → [type "gpt"] → [GPT-4o + GPT-4o Mini visible]
→ [select GPT-4o Mini] → Complete
```

### Invalid Key Flow

```
KeyInput → [enter "bad-key"] → [next]
→ [error: "Invalid API key format"]
→ [key stays, cannot advance]
```

### Fetch Failure Flow

```
KeyInput → [enter valid key] → [fetch initiated]
→ [fetch fails] → [fallback to hardcoded models]
→ Can still proceed to ModelSelect with hardcoded list
```

### Add Another Provider Flow

```
Complete → [select "yes"] → ProviderSelect
→ [configure new provider]
→ Complete → [select "no"] → Done
```

### Esc/Back Navigation Flow

```
Complete → [back] → ModelSelect → [back] → KeyInput
→ [back] → ProviderSelect → [back] → Welcome
```

### Empty Filter Flow

```
ProviderSelect → [type "xyz"] → [no matches]
→ [filtered_provider_indices empty]
→ [get_filtered_provider_count() = 0]
→ Cannot select (no valid items)
```

### Multiple Provider Configuration Flow

```
Complete → [yes] → ProviderSelect → [select Anthropic]
→ KeyInput → [enter sk-ant-...] → ModelSelect → [select Claude]
→ Complete → [no] → Done
Provider list contains OpenAI + Anthropic configurations
```

---

## Test Execution Matrix

| Category | Test Count | Priority |
|----------|------------|----------|
| Step Transitions | 5 | P0 |
| Navigation & Input | 6 | P0 |
| Provider Selection | 5 | P0 |
| API Key Handling | 8 | P0 |
| Model Fetching & Selection | 6 | P1 |
| Settings Persistence | 4 | P1 |
| Error Handling | 5 | P1 |
| Multi-Provider Flow | 5 | P2 |
| Edge Cases | 10 | P2 |
| Async Behavior Tests | 7 | P1 |
| **Total** | **61** | |

---

## Appendix: Provider Key Prefixes

| Provider | Key Prefix | Validation |
|----------|------------|------------|
| OpenAI | `sk-` | Must start with `sk-` |
| Anthropic | `sk-ant-` | Must start with `sk-ant-` |
| Google | (empty) | Any non-empty |
| Cohere | (empty) | Any non-empty |
| Mistral | (empty) | Any non-empty |
| DeepSeek | (empty) | Any non-empty |
| Groq | (empty) | Any non-empty |
| OpenRouter | (empty) | Any non-empty |
| HuggingFace | (empty) | Any non-empty |
| xAI | (empty) | Any non-empty |
| Azure | (empty) | Any non-empty |
| Moonshot | (empty) | Any non-empty |
| Perplexity | (empty) | Any non-empty |
| Ollama | (empty) | Any non-empty |
| Hyperbolic | (empty) | Any non-empty |
| Together | (empty) | Any non-empty |
| ZAI | (empty) | Any non-empty |
| MiniMax | (empty) | Any non-empty |
| Mira | (empty) | Any non-empty |
| Galadriel | (empty) | Any non-empty |
| Llamafile | (empty) | Any non-empty |

---

## Appendix: Hardcoded Model Lists by Provider

| Provider | Model Count | Sample Models |
|----------|-------------|---------------|
| OpenAI | 3 | GPT-4o, GPT-4o Mini, O1 Mini |
| Anthropic | 3 | Claude Sonnet 4, Claude Haiku, Claude Opus |
| Google | 2 | Gemini Pro, Gemini Flash |
| Cohere | 2 | Command R, Command R Plus |
| Mistral | 1 | Mistral Large |
| DeepSeek | 1 | DeepSeek Chat |
| Groq | 1 | Llama 3.1 8B Instant |
| OpenRouter | 1 | GPT-4o (openai/gpt-4o) |
| HuggingFace | 1 | Llama 2 70B |
| xAI | 1 | Grok Beta |
| Azure | 1 | GPT-4o |
| Moonshot | 1 | Moonshot V1 8K |
| Perplexity | 1 | Llama 3.1 Sonar Large |
| Ollama | 1 | Llama 3.2 |
| Hyperbolic | 1 | Llama 3.1 70B |
| Together | 1 | Llama 3.2 3B Turbo |
| ZAI | 1 | Default |
| MiniMax | 1 | ABAB 6.5 |
| Mira | 1 | Default |
| Galadriel | 1 | Default |
| Llamafile | 1 | Llamafile |

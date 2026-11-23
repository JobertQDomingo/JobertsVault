// Example steps file: shows the driver's API (check, invoke, screenshots, cleanup).
// Phase 4's check of card spending levels (CC-10, CC-11). DEV database only.
// Adds a made-up "t55 Card" and its expenses, and removes them after.
export default async (app) => {
  const check = (name, cond, extra = "") => app.log((cond ? "PASS " : "FAIL ") + name + (extra ? " — " + extra : ""));
  const { decimalsLocked, ...original } = await app.invoke("get_settings");
  await app.invoke("update_settings", { settings: { ...original, onboardingCompleted: true } });
  const { today } = await app.invoke("get_transaction_defaults");
  const cats = (await app.invoke("list_categories")).categories;
  const food = cats.find((c) => c.kind === "expense" && !c.archived && c.name.startsWith("Food")) ?? cats.find((c) => c.kind === "expense" && !c.archived);
  let cardId = null;
  const rows = async () => cardId === null ? [] : (await app.invoke("list_transactions", { filter: { period: { kind: "custom", start: "2026-01-01", end: "2027-12-31" }, kind: null, accountId: cardId, categoryId: null, paymentMethodId: null, search: null }, page: 0 })).items;
  const shots = async (name) => {
    await app.shot(name + "-light");
    await app.theme("dark");
    await app.shot(name + "-dark");
    await app.theme("light");
  };
  const home = async () => {
    await app.eval(`location.href = "/"`);
    await app.sleep(1800);
  };
  const toasts = () => app.eval(`[...document.querySelectorAll("[data-sonner-toast]")].map(t => t.innerText.trim())`);
  // Quick-add an expense on the card; Enter in the amount saves.
  const quickAdd = async (amount) => {
    await app.key("n", ["Control"]);
    await app.waitFor(`document.querySelector("[role=dialog] h2")?.textContent === "Add transaction"`, 5000);
    await app.sleep(300);
    await app.eval(`document.getElementById("qa-amount").focus(); true`);
    await app.type(amount);
    await app.eval(`document.getElementById("qa-accountId").focus(); true`);
    await app.type("t55 Card");
    await app.sleep(300);
    await app.key("Enter");
    await app.eval(`document.getElementById("qa-category").focus(); true`);
    await app.type(food.name.slice(0, 4));
    await app.sleep(300);
    await app.key("Enter");
    await app.eval(`document.getElementById("qa-amount").focus(); true`);
    await app.key("Enter");
  };
  try {
    await app.theme("light");
    const card = await app.invoke("create_account", { account: {
      kind: "credit_card", name: "t55 Card", openingBalance: 0, creditLimit: 2000000, statementDay: 28, dueDay: 12,
      spendSoft: null, spendMid: null, spendHard: null,
    } });
    cardId = card.id;
    check("a card sent without levels gets 3,000 / 4,000 / 5,000", card.spendSoft === 300000 && card.spendMid === 400000 && card.spendHard === 500000, JSON.stringify([card.spendSoft, card.spendMid, card.spendHard]));
    check("an existing card got the defaults from 0004", (await app.invoke("list_accounts")).accounts.filter((a) => a.kind === "credit_card" && a.id !== cardId).every((a) => a.spendSoft === 300000 && a.spendHard === 500000));
    await app.invoke("create_transaction", { transaction: { kind: "expense", amount: 200000, date: today, accountId: cardId, toAccountId: null, categoryId: food.id, payee: null, paymentMethodId: null, note: "t55" } });

    // The card form's levels row.
    await home();
    await app.click({ text: "Accounts", selector: "nav a" });
    await app.waitFor(`document.body.innerText.includes("t55 Card")`, 5000);
    await app.click({ text: "New account" });
    await app.waitFor(`!!document.getElementById("acct-kind")`, 5000);
    await app.eval(`document.getElementById("acct-name").focus(); true`);
    await app.type("Sample card");
    await app.click("#acct-kind");
    await app.click({ text: "Credit card", selector: "[role=option]" });
    await app.sleep(300);
    const levels = await app.eval(`["acct-spendSoft","acct-spendMid","acct-spendHard"].map(id => document.getElementById(id)?.value)`);
    check("a new card's form starts at 3,000.00 / 4,000.00 / 5,000.00", JSON.stringify(levels) === JSON.stringify(["3,000.00", "4,000.00", "5,000.00"]), JSON.stringify(levels));
    await shots("t55-form-levels");
    await app.key("Escape");
    await app.sleep(300);

    // ₱2,000 + ₱1,240 = ₱3,240: the soft level, reached now, saves with a warning.
    await home();
    await quickAdd("1240");
    await app.waitFor(`document.body.innerText.includes("past your")`, 8000);
    const soft = (await toasts()).find((t) => t.includes("past your")) ?? "";
    check("soft reached: the warning toast", soft === "t55 Card: ₱3,240.00 this cycle, past your ₱3,000.00 soft level.", soft);
    check("soft reached: it saved", (await rows()).length === 2);
    await shots("t55-soft-toast");
    await app.sleep(500);
    const softCard = await app.eval(`(() => { const p = [...document.querySelectorAll("p.truncate")].find(p => p.textContent === "t55 Card"); if (!p) return ""; p.scrollIntoView({ block: "center" }); return p.parentElement.innerText; })()`);
    check("the dashboard card summary shows Soft level", softCard.includes("Soft level"), softCard.replace(/\s+/g, " "));

    // A second expense that stays at soft: no warning.
    await app.sleep(8500);
    await quickAdd("100");
    await app.waitFor(`document.body.innerText.includes("Saved")`, 8000);
    await app.sleep(500);
    check("already at soft: no new warning", !(await toasts()).some((t) => t.includes("past your")), JSON.stringify(await toasts()));

    // ₱3,340 + ₱1,780 = ₱5,120: the hard level asks first; Go back keeps the form.
    await app.sleep(4500);
    await quickAdd("1780");
    await app.waitFor(`[...document.querySelectorAll("[role=dialog] h2")].some(h => h.textContent === "Spend anyway?")`, 8000);
    await app.sleep(300);
    const message = await app.eval(`[...document.querySelectorAll("[role=dialog]")].find(d => d.querySelector("h2")?.textContent === "Spend anyway?").innerText`);
    check("hard: the confirm's message", message.includes("This takes t55 Card to ₱5,120.00 this cycle, past your ₱5,000.00 hard level."), message.replace(/\s+/g, " "));
    await shots("t55-spend-anyway");
    await app.click({ text: "Go back" });
    await app.sleep(500);
    check("Go back: nothing saved", (await rows()).length === 3);
    check("Go back: the form keeps its amount", (await app.eval(`document.getElementById("qa-amount")?.value`)) === "1,780.00");
    await app.eval(`document.getElementById("qa-amount").focus(); true`);
    await app.key("Enter");
    await app.waitFor(`[...document.querySelectorAll("[role=dialog] h2")].some(h => h.textContent === "Spend anyway?")`, 8000);
    await app.sleep(300);
    await app.click({ text: "Spend anyway" });
    await app.waitFor(`!document.querySelector("[role=dialog]")`, 8000);
    check("Spend anyway: it saved", (await rows()).length === 4);
    await app.sleep(4500);
    const hardCard = await app.eval(`(() => { const p = [...document.querySelectorAll("p.truncate")].find(p => p.textContent === "t55 Card"); if (!p) return ""; p.scrollIntoView({ block: "center" }); return p.parentElement.innerText; })()`);
    check("the dashboard card summary shows Hard level", hardCard.includes("Hard level"), hardCard.replace(/\s+/g, " "));
    await app.sleep(400);
    await shots("t55-dashboard-hard");

    // The level on the Accounts list and the card's details.
    await app.click({ text: "Accounts", selector: "nav a" });
    await app.waitFor(`document.body.innerText.includes("t55 Card")`, 5000);
    await app.sleep(400);
    const row = await app.eval(`[...document.querySelectorAll("a[href^='/accounts/']")].find(a => a.innerText.startsWith("t55 Card"))?.innerText ?? ""`);
    check("the Accounts row shows Hard level", row.includes("Hard level"), row.replace(/\s+/g, " "));
    await shots("t55-accounts-hard");
    await app.click({ text: "t55 Card", exact: false, selector: "a[href^='/accounts/'] p" });
    await app.waitFor(`document.body.innerText.includes("Spending level")`, 5000);
    await app.sleep(400);
    const table = await app.eval(`document.querySelector("table")?.innerText ?? ""`);
    check("the details show the level under This cycle", /₱5,120\.00 spent\s+Spending level\s+Past your ₱5,000\.00 hard level/.test(table), table.replace(/\s+/g, " "));
    await shots("t55-details-hard");
    check("no console errors", app.errors().length === 0, app.errors().join(" / "));
  } finally {
    for (const r of await rows().catch(() => [])) await app.invoke("delete_transaction", { id: r.id }).catch(() => {});
    if (cardId !== null) await app.invoke("delete_account", { id: cardId }).catch((e) => app.log("delete failed " + JSON.stringify(e)));
    await app.invoke("update_settings", { settings: original });
    const left = (await app.invoke("list_accounts")).accounts.filter((a) => a.name.startsWith("t55"));
    app.log("cleaned up; t55 accounts left: " + left.length);
  }
};

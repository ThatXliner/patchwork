// Legacy code that needs cleanup
const oldApi = {
    fetchData: (id) => {
        console.log("fetching", id);
        return { id, name: "test", value: 42 };
    },
    saveData: (data) => {
        console.log("saving", data);
        return true;
    }
};

function processItem(id) {
    const { id: itemId, name: itemName } = id;
    fetchOldData(id);
    saveData(itemName);
    console.log("processed", itemId);
    return itemId;
}

function saveAll(items) {
    for (const item of items) {
        const { id, name } = item;
        saveData(item);
    }
}

function showError(context, msg) {
    console.log("error", msg);
}

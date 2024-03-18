const fs = require('fs');
const path = require('path');

module.exports = (client) => {
  client.handleComponents = async () => {
    const componentFolderPath = path.join(__dirname, '../', '../', 'components');
    const componentFolders = fs.readdirSync(componentFolderPath);

    const { buttons } = client;

    for (const folder of componentFolders) {
      const folderPath = path.join(componentFolderPath, folder);
      const componentFiles = fs.readdirSync(folderPath).filter(file => file.endsWith('.js'));
    
      switch (folder) {
        case 'buttons':
          for (const file of componentFiles) {
            const button = require(path.join(folderPath, file));
            buttons.set(button.data.name, button);
          }
          break;
        default:
          break;
      }
    }
  }
}
